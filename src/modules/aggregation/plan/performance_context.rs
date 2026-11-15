use std::collections::BTreeSet;

use crate::ast::{Program, TopLevelDecl};

pub(crate) fn program_with_performance_context(program: &Program, context: &Program) -> Program {
    let mut declarations = program.declarations.clone();
    declarations.extend(
        context
            .declarations
            .iter()
            .filter(|declaration| matches!(declaration, TopLevelDecl::Import(_)))
            .cloned(),
    );
    let mut known: BTreeSet<String> =
        declarations.iter().filter_map(context_declaration_identity).collect();
    declarations.extend(
        context
            .declarations
            .iter()
            .filter(|declaration| context_declaration_identity(declaration).is_some())
            .filter(|declaration| {
                context_declaration_identity(declaration)
                    .is_some_and(|identity| known.insert(identity))
            })
            .cloned(),
    );
    Program { file_metadata: program.file_metadata.clone(), declarations }
}

fn context_declaration_identity(declaration: &TopLevelDecl) -> Option<String> {
    let (kind, name) = match declaration {
        TopLevelDecl::Constant(value) => ("constant", value.name.clone()),
        TopLevelDecl::Verb(value) => ("verb", value.name.clone()),
        TopLevelDecl::ExternalVerb(value) => ("verb", value.name.clone()),
        TopLevelDecl::Struct(value) => ("struct", value.name.clone()),
        TopLevelDecl::Pack(value) => ("pack", value.name.clone()),
        TopLevelDecl::Serialize(value) => ("serialize", value.name.clone()),
        TopLevelDecl::Enum(value) => ("enum", value.name.clone()),
        TopLevelDecl::Role(value) => ("role", value.name.clone()),
        TopLevelDecl::Perform(value) => {
            return Some(format!("perform:{}:{}", value.role_name, value.target.spelling_key()));
        }
        TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => return None,
    };
    Some(format!("{kind}:{name}"))
}

#[cfg(test)]
mod tests {
    use super::program_with_performance_context;
    use crate::lexer::scan;
    use crate::parser::parse;

    #[test]
    fn contextual_program_matches_two_unit_plan_context() {
        let unit = parse(scan("verb unit() -> Int { return 1; }\n").0).expect("unit parses");
        let root = parse(scan("struct Root { value: Int, }\n").0).expect("root parses");

        let contextual = program_with_performance_context(&unit, &root);

        assert_eq!(contextual.declarations.len(), unit.declarations.len() + 1);
        assert_eq!(contextual.declarations[0], unit.declarations[0]);
        assert_eq!(contextual.declarations[1], root.declarations[0]);
    }
}
