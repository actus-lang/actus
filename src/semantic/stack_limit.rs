use super::errors::{SemanticError, SemanticErrorKind};

pub fn validate_stack_limit_backend_target(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
    backend: &str,
) -> Result<(), SemanticError> {
    let has_stack_limit = program.declarations.iter().any(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => verb
            .metadata
            .iter()
            .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::StackLimit(_))),
        crate::ast::TopLevelDecl::ExternalVerb(verb) => verb
            .metadata
            .iter()
            .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::StackLimit(_))),
        crate::ast::TopLevelDecl::Perform(perform) => perform.methods.iter().any(|method| {
            method
                .metadata
                .iter()
                .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::StackLimit(_)))
        }),
        crate::ast::TopLevelDecl::Role(role) => role.methods.iter().any(|method| {
            method
                .metadata
                .iter()
                .any(|attribute| matches!(attribute, crate::ast::MetaAttribute::StackLimit(_)))
        }),
        _ => false,
    });
    if !has_stack_limit {
        return Ok(());
    }
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
        span: crate::lexer::SourceSpan::new(0, 0),
    })
}
