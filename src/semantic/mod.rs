mod analyzer;
mod argument_shapes;
mod array_capacity;
mod borrowing;
mod call_arguments;
mod calls;
mod cleanup;
mod dynamic;
mod enum_constructors;
mod enum_registry;
mod enums;
mod errors;
mod generated_locals;
mod generic_cache;
mod generic_calls;
mod generic_errors;
mod generics;
mod intrinsics;
mod loans;
mod methods;
mod model;
mod origins;
mod ownership;
mod packs;
mod pattern_moves;
mod pattern_support;
mod patterns;
mod print_validation;
mod repr;
mod roles;
mod scopes;
mod serialization;
mod state;
mod structs;
mod type_substitution;
mod types;
mod volatile;

pub use analyzer::analyze;
pub(crate) use analyzer::validate_bounded_recursion;
pub(crate) use analyzer::{
    validate_stack_limit_dynamic_dispatch, validate_stack_limit_native_claims,
    validate_stack_limit_recursion,
};
pub(crate) use calls::parse_type_name_key;
pub use cleanup::{CleanupAction, LoopExitKind, LoopUnwindPlan, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub(crate) use generated_locals::normalize_program as normalize_generated_locals;
pub use model::{
    ArgumentRoleFact, ArgumentRoleSource, Binding, BorrowRecord, ConditionalFact, DynamicRoleType,
    ExclusiveLoan, ExplicitDrop, FatPointerLayout, GenericInstance, LiteralFact, Origin,
    OriginRecord, OriginRoot, PackFieldContract, PackLayoutContract, ReachablePerformance,
    SemanticModel, SerializationContract,
};
pub use state::{AccessState, OwnershipState, ResourceState};
pub use types::{SemanticType, TypeRegistry};

pub fn filter_program_for_target(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
) -> crate::ast::Program {
    crate::ast::Program {
        file_metadata: program.file_metadata.clone(),
        declarations: program
            .declarations
            .iter()
            .filter(|declaration| declaration_matches_target(declaration, target))
            .cloned()
            .collect(),
    }
}

pub fn filter_program_for_target_checked(
    program: &crate::ast::Program,
    target: &crate::target::TargetSpec,
) -> Result<crate::ast::Program, SemanticError> {
    let filtered = filter_program_for_target(program, target);
    let mut declarations = Vec::new();
    for declaration in &filtered.declarations {
        match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => declarations.push((&verb.metadata, verb.span)),
            crate::ast::TopLevelDecl::ExternalVerb(verb) => {
                declarations.push((&verb.metadata, verb.span));
            }
            crate::ast::TopLevelDecl::Role(role) => {
                declarations
                    .extend(role.methods.iter().map(|method| (&method.metadata, method.span)));
            }
            crate::ast::TopLevelDecl::Perform(perform) => {
                declarations
                    .extend(perform.methods.iter().map(|method| (&method.metadata, method.span)));
            }
            _ => {}
        }
    }
    for (metadata, span) in declarations {
        for attribute in metadata {
            let crate::ast::MetaAttribute::Capability(capability) = attribute else { continue };
            if !target.supports_capability(*capability) {
                return Err(SemanticError {
                    kind: crate::semantic::SemanticErrorKind::UnsupportedCapability {
                        capability: capability.name().to_owned(),
                        target: target.triple().to_string(),
                    },
                    span,
                });
            }
        }
    }
    Ok(filtered)
}

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

fn declaration_matches_target(
    declaration: &crate::ast::TopLevelDecl,
    target: &crate::target::TargetSpec,
) -> bool {
    let metadata = match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => &verb.metadata,
        crate::ast::TopLevelDecl::ExternalVerb(verb) => &verb.metadata,
        _ => return true,
    };
    metadata.iter().all(|attribute| match attribute {
        crate::ast::MetaAttribute::Target(selector) => target.matches_platform(selector),
        crate::ast::MetaAttribute::Test => true,
        crate::ast::MetaAttribute::Fault(_) => true,
        crate::ast::MetaAttribute::Limitless(_) => true,
        crate::ast::MetaAttribute::MustUse(_) => true,
        crate::ast::MetaAttribute::Effect(_) => true,
        crate::ast::MetaAttribute::Deterministic => true,
        crate::ast::MetaAttribute::Realtime => true,
        crate::ast::MetaAttribute::Bounded(_) => true,
        crate::ast::MetaAttribute::StackLimit(_) => true,
        crate::ast::MetaAttribute::Capability(_) => true,
        crate::ast::MetaAttribute::Repr(_) => true,
        crate::ast::MetaAttribute::Volatile => true,
        crate::ast::MetaAttribute::ZeroOnDrop => true,
    })
}
pub(crate) use type_substitution::TypeSubstitution;
