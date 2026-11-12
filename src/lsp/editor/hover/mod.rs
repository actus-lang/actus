mod formatting;
mod lookup;
mod model;
mod tokens;

use super::position::LspRange;

#[derive(Clone, Debug)]
pub struct HoverInfo {
    pub contents: String,
    pub range: LspRange,
}

pub(super) use formatting::contract_documentation;

pub(super) fn contract_documentation_for_name(
    program: &crate::ast::Program,
    name: &str,
) -> Option<String> {
    program.declarations.iter().find_map(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) if verb.name == name => {
            formatting::contract_documentation(verb.doc.as_deref(), verb.contract.as_ref())
        }
        crate::ast::TopLevelDecl::ExternalVerb(verb) if verb.name == name => {
            formatting::contract_documentation(verb.doc.as_deref(), verb.contract.as_ref())
        }
        _ => None,
    })
}
pub(super) use lookup::find_hover;
