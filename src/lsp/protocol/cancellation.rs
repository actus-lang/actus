use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde_json::Value;

#[derive(Clone, Default)]
pub(super) struct CancellationRegistry {
    canceled: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone)]
pub(super) struct CancellationToken {
    key: String,
    registry: CancellationRegistry,
}

impl CancellationRegistry {
    pub(super) fn token(&self, id: Option<&Value>) -> Option<CancellationToken> {
        id.map(|id| CancellationToken { key: id.to_string(), registry: self.clone() })
    }

    pub(super) fn cancel(&self, id: &Value) {
        if let Ok(mut canceled) = self.canceled.lock() {
            canceled.insert(id.to_string());
        }
    }

    fn clear(&self, key: &str) {
        if let Ok(mut canceled) = self.canceled.lock() {
            canceled.remove(key);
        }
    }
}

impl CancellationToken {
    pub(super) fn is_canceled(&self) -> bool {
        self.registry.canceled.lock().map(|canceled| canceled.contains(&self.key)).unwrap_or(true)
    }

    pub(super) fn checkpoint(&self) -> bool {
        self.is_canceled()
    }
}

impl Drop for CancellationToken {
    fn drop(&mut self) {
        self.registry.clear(&self.key);
    }
}
