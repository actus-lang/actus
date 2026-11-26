use super::errors::{SemanticError, SemanticErrorKind};
use crate::ast::{MetaAttribute, TopLevelDecl};
use crate::lexer::SourceSpan;

fn claims_stack_limit(metadata: &[MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::StackLimit(_)))
}

/// The span of the first declaration that carries `meta stack_limit`: the verb,
/// the external verb, or the method of a `perform` or a role.
fn first_stack_limit_span(program: &crate::ast::Program) -> Option<SourceSpan> {
    program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) => claims_stack_limit(&verb.metadata).then_some(verb.span),
        TopLevelDecl::ExternalVerb(verb) => claims_stack_limit(&verb.metadata).then_some(verb.span),
        TopLevelDecl::Perform(perform) => perform
            .methods
            .iter()
            .find(|method| claims_stack_limit(&method.metadata))
            .map(|method| method.span),
        TopLevelDecl::Role(role) => role
            .methods
            .iter()
            .find(|method| claims_stack_limit(&method.metadata))
            .map(|method| method.span),
        _ => None,
    })
}

/// Rejects `meta stack_limit` when the backend cannot prove it for the target.
/// The error points at the declaration that carries the contract.
pub fn validate_stack_limit_backend_target(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
    backend: &str,
) -> Result<(), SemanticError> {
    let Some(span) = first_stack_limit_span(program) else {
        return Ok(());
    };
    let target_name = target.triple().to_string();
    let supported_target =
        matches!(target_name.as_str(), "x86_64-unknown-linux-gnu" | "thumbv7em-none-eabi");
    if backend == "llvm" && supported_target {
        return Ok(());
    }
    Err(SemanticError {
        kind: SemanticErrorKind::UnsupportedStackLimitTarget {
            backend: backend.to_owned(),
            target: target_name,
        },
        span,
    })
}
