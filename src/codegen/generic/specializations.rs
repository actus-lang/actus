use std::collections::HashSet;

use crate::ast::TopLevelDecl;

/// One generic verb reached through two bindings (`ie::train` and the
/// unqualified `train`, or the unit's own definition and an alias of it)
/// specializes to one instance name. Declaration names are unique in a
/// program (`E1110`), so these are the same instance: the unit's own
/// specialized definition wins, and an imported instance is kept once.
pub(super) fn unique_instances(
    declarations: Vec<TopLevelDecl>,
    external_instances: &HashSet<String>,
) -> Vec<TopLevelDecl> {
    let defined = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some(verb.name.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let mut kept = HashSet::new();
    declarations
        .into_iter()
        .filter(|declaration| match declaration {
            TopLevelDecl::ExternalVerb(verb) if external_instances.contains(&verb.name) => {
                !defined.contains(&verb.name) && kept.insert(verb.name.clone())
            }
            _ => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarations(source: &str) -> Vec<TopLevelDecl> {
        let (tokens, errors) = crate::lexer::scan(source);
        assert!(errors.is_empty(), "{errors:?}");
        crate::parser::parse(tokens).expect("fixture parses").declarations
    }

    fn names(declarations: &[TopLevelDecl]) -> Vec<String> {
        declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Verb(verb) => Some(format!("verb {}", verb.name)),
                TopLevelDecl::ExternalVerb(verb) => Some(format!("extern {}", verb.name)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn one_instance_reached_through_two_bindings_is_kept_once() {
        let program = declarations(
            "unsafe extern \"C\" verb gen__64() -> Int; \
             unsafe extern \"C\" verb gen__64() -> Int; \
             unsafe extern \"C\" verb own__64() -> Int; \
             verb own__64() -> Int { return 0; }",
        );
        let instances = HashSet::from(["gen__64".to_owned(), "own__64".to_owned()]);
        assert_eq!(
            names(&unique_instances(program, &instances)),
            ["extern gen__64", "verb own__64"]
        );
    }

    #[test]
    fn declarations_that_are_not_instances_are_untouched() {
        let program = declarations(
            "unsafe extern \"C\" verb write() -> Int; unsafe extern \"C\" verb write() -> Int;",
        );
        assert_eq!(
            names(&unique_instances(program, &HashSet::new())),
            ["extern write", "extern write"]
        );
    }
}
