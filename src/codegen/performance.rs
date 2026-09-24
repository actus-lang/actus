use std::collections::HashSet;

use crate::semantic::ReachablePerformance;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PerformanceImplementation {
    pub(super) role_name: String,
    pub(super) target_type: String,
    pub(super) method_name: String,
    pub(super) symbol: String,
}

#[derive(Default)]
pub(super) struct PerformanceRegistry {
    implementations: Vec<PerformanceImplementation>,
}

impl PerformanceRegistry {
    pub(super) fn from_reachable(reachable: &[ReachablePerformance]) -> Self {
        let mut implementations = reachable
            .iter()
            .map(|implementation| PerformanceImplementation {
                role_name: implementation.role_name.clone(),
                target_type: implementation.target_type.clone(),
                method_name: implementation.method_name.clone(),
                symbol: performance_symbol(
                    &implementation.role_name,
                    &implementation.target_type,
                    &implementation.method_name,
                ),
            })
            .collect::<Vec<_>>();
        implementations.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        Self { implementations }
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        let mut symbols = HashSet::new();
        for implementation in &self.implementations {
            if !symbols.insert(&implementation.symbol) {
                return Err(format!("duplicate performance symbol `{}`", implementation.symbol));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn implementations(&self) -> &[PerformanceImplementation] {
        &self.implementations
    }
}

fn performance_symbol(role: &str, target: &str, method: &str) -> String {
    format!(
        "actus_perf_{}_{}_{}",
        encode_component(role),
        encode_component(target),
        encode_component(method)
    )
}

fn encode_component(component: &str) -> String {
    component.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() {
            encoded.push(byte as char);
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02x}"));
        }
        encoded
    })
}

#[cfg(test)]
mod tests {
    use super::{PerformanceRegistry, performance_symbol};
    use crate::semantic::ReachablePerformance;

    #[test]
    fn creates_stable_symbols_for_simple_performances() {
        assert_eq!(performance_symbol("Writer", "File", "write"), "actus_perf_Writer_File_write");
    }

    #[test]
    fn encodes_type_applications_without_symbol_collisions() {
        let first = performance_symbol("Writer", "Box[Int]", "write");
        let second = performance_symbol("Writer", "Box_BInt", "write");
        assert_ne!(first, second);
    }

    #[test]
    fn sorts_reachable_implementations_by_symbol() {
        let registry = PerformanceRegistry::from_reachable(&[
            ReachablePerformance {
                role_name: "Reader".to_owned(),
                target_type: "File".to_owned(),
                method_name: "read".to_owned(),
            },
            ReachablePerformance {
                role_name: "Writer".to_owned(),
                target_type: "File".to_owned(),
                method_name: "write".to_owned(),
            },
        ]);
        assert_eq!(registry.implementations()[0].symbol, "actus_perf_Reader_File_read");
        assert!(registry.validate().is_ok());
    }
}
