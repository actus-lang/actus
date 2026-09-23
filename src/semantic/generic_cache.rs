use std::collections::BTreeMap;

use super::model::GenericInstance;

pub(super) struct GenericInstanceCache {
    toolchain_hash: String,
    entries: BTreeMap<(String, String), GenericInstance>,
}

impl GenericInstanceCache {
    pub(super) fn new(toolchain_hash: impl Into<String>) -> Self {
        Self { toolchain_hash: toolchain_hash.into(), entries: BTreeMap::new() }
    }

    pub(super) fn for_current_toolchain() -> Self {
        let identity = option_env!("ACTUS_TOOLCHAIN_HASH").unwrap_or(env!("CARGO_PKG_VERSION"));
        Self::new(stable_digest(identity))
    }

    pub(super) fn insert(&mut self, instance: GenericInstance) {
        let key = (self.toolchain_hash.clone(), instance.canonical_key.clone());
        self.entries.entry(key).or_insert(instance);
    }

    pub(super) fn into_instances(self) -> Vec<GenericInstance> {
        self.entries.into_values().collect()
    }

    #[cfg(test)]
    fn contains(&self, canonical_key: &str) -> bool {
        self.entries.keys().any(|(_, key)| key == canonical_key)
    }
}

fn stable_digest(input: &str) -> String {
    let mut digest = 0xcbf29ce484222325_u64;
    for byte in input.as_bytes() {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x100000001b3);
    }
    format!("{digest:016x}")
}

#[cfg(test)]
mod tests {
    use super::GenericInstanceCache;
    use crate::ast::TypeName;
    use crate::lexer::SourceSpan;
    use crate::semantic::GenericInstance;

    fn instance(key: &str) -> GenericInstance {
        GenericInstance {
            name: "Box".to_owned(),
            arguments: vec![TypeName {
                name: "Int".to_owned(),
                arguments: Vec::new(),
                span: SourceSpan::new(0, 0),
            }],
            canonical_key: key.to_owned(),
        }
    }

    #[test]
    fn deduplicates_by_canonical_key_within_a_toolchain() {
        let mut cache = GenericInstanceCache::new("toolchain-a");
        cache.insert(instance("Box[Int]"));
        cache.insert(instance("Box[Int]"));
        assert!(cache.contains("Box[Int]"));
        assert_eq!(cache.into_instances().len(), 1);
    }

    #[test]
    fn different_toolchain_caches_are_independent() {
        let mut first = GenericInstanceCache::new("toolchain-a");
        let mut second = GenericInstanceCache::new("toolchain-b");
        first.insert(instance("Box[Int]"));
        second.insert(instance("Box[Int]"));
        assert_eq!(first.into_instances().len(), 1);
        assert_eq!(second.into_instances().len(), 1);
    }
}
