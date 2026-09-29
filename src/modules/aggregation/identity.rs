use crate::modules::ModuleResolutionError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleNamespace {
    module_path: String,
    symbol_prefix: String,
}

impl ModuleNamespace {
    pub fn root() -> Self {
        Self { module_path: "<root>".to_owned(), symbol_prefix: "actus_root".to_owned() }
    }

    pub fn from_module_path(module_path: &str) -> Result<Self, ModuleResolutionError> {
        let segments = module_path.split("::").collect::<Vec<_>>();
        if segments.is_empty() || segments.iter().any(|segment| !valid_segment(segment)) {
            return Err(ModuleResolutionError::InvalidPath(module_path.to_owned()));
        }
        let canonical_path = segments.join("::");
        let symbol_prefix = segments
            .iter()
            .map(|segment| format!("{}_{}", segment.len(), segment))
            .collect::<Vec<_>>()
            .join("_");
        Ok(Self {
            module_path: canonical_path,
            symbol_prefix: format!("actus_mod_{symbol_prefix}"),
        })
    }

    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    pub fn symbol_prefix(&self) -> &str {
        &self.symbol_prefix
    }
}

fn valid_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    let Some(first) = characters.next() else { return false };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::ModuleNamespace;

    #[test]
    fn creates_stable_length_delimited_symbol_prefixes() {
        let namespace = ModuleNamespace::from_module_path("graphics::math").unwrap();
        assert_eq!(namespace.module_path(), "graphics::math");
        assert_eq!(namespace.symbol_prefix(), "actus_mod_8_graphics_4_math");
    }

    #[test]
    fn distinguishes_segment_boundaries_without_separator_ambiguity() {
        let first = ModuleNamespace::from_module_path("ab::c").unwrap();
        let second = ModuleNamespace::from_module_path("a::bc").unwrap();
        assert_ne!(first.symbol_prefix(), second.symbol_prefix());
    }

    #[test]
    fn rejects_invalid_and_empty_module_paths() {
        for path in ["", "graphics:", "graphics::::math", "graphics/math", "9graphics"] {
            assert!(ModuleNamespace::from_module_path(path).is_err(), "accepted `{path}`");
        }
    }
}
