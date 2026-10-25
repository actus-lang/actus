use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendSymbolBindings {
    external: BTreeMap<(String, String), String>,
    unique_source: BTreeMap<String, String>,
}

impl BackendSymbolBindings {
    pub fn new(bindings: impl IntoIterator<Item = (String, String, String)>) -> Self {
        Self::new_with_unique_source_policy(bindings, false)
    }

    pub(crate) fn new_prefer_first_unique_source(
        bindings: impl IntoIterator<Item = (String, String, String)>,
    ) -> Self {
        Self::new_with_unique_source_policy(bindings, true)
    }

    fn new_with_unique_source_policy(
        bindings: impl IntoIterator<Item = (String, String, String)>,
        prefer_first: bool,
    ) -> Self {
        let mut external = BTreeMap::new();
        let mut unique_source = BTreeMap::new();
        for (namespace, source, symbol) in bindings {
            external.insert((namespace, source.clone()), symbol.clone());
            match unique_source.get(&source) {
                None => {
                    unique_source.insert(source, symbol);
                }
                Some(existing) if existing == &symbol => {}
                Some(_) if !prefer_first => {
                    unique_source.remove(&source);
                }
                Some(_) => {}
            }
        }
        Self { external, unique_source }
    }

    pub(crate) fn external_symbol<'a>(&'a self, namespace: &str, source_name: &'a str) -> &'a str {
        self.external
            .get(&(namespace.to_owned(), source_name.to_owned()))
            .map(String::as_str)
            .or_else(|| self.unique_source.get(source_name).map(String::as_str))
            .unwrap_or(source_name)
    }
}
