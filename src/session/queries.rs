use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock};

/// Session-scoped memoization with one computation per key revision.
///
/// The map lock is held only while a slot is looked up or replaced. The
/// computation itself runs outside that lock, so different keys are computed
/// in parallel, while concurrent callers of one key wait for a single result.
pub(crate) struct QueryCache<K, V> {
    slots: Mutex<HashMap<K, Arc<OnceLock<V>>>>,
}

impl<K: Clone + Eq + Hash, V: Clone> QueryCache<K, V> {
    pub(crate) fn new() -> Self {
        Self { slots: Mutex::new(HashMap::new()) }
    }

    /// Returns the published value for `key` when `is_current` accepts it.
    /// Otherwise the value is computed once and published for later callers.
    pub(crate) fn get_or_compute(
        &self,
        key: &K,
        is_current: impl Fn(&V) -> bool,
        compute: impl FnOnce() -> V,
    ) -> V {
        let slot = self.slot(key, &is_current);
        let mut compute = Some(compute);
        let value = slot.get_or_init(|| compute.take().expect("query computes once")());
        if is_current(value) {
            return value.clone();
        }
        // A concurrent caller published another revision of this key; this
        // caller's revision is computed without replacing the published one.
        compute.map_or_else(|| value.clone(), |compute| compute())
    }

    fn slot(&self, key: &K, is_current: &impl Fn(&V) -> bool) -> Arc<OnceLock<V>> {
        let mut slots = self.slots.lock().expect("query cache lock poisoned");
        if let Some(slot) = slots.get(key)
            && slot.get().is_none_or(is_current)
        {
            return Arc::clone(slot);
        }
        let slot = Arc::new(OnceLock::new());
        slots.insert(key.clone(), Arc::clone(&slot));
        slot
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slots.lock().expect("query cache lock poisoned").len()
    }
}
