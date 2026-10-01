mod analyzer;
mod argument_shapes;
mod borrowing;
mod call_arguments;
mod calls;
mod cleanup;
mod dynamic;
mod enum_constructors;
mod enum_registry;
mod enums;
mod errors;
mod generic_cache;
mod generic_calls;
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
mod roles;
mod scopes;
mod state;
mod structs;
mod type_substitution;
mod types;

pub use analyzer::analyze;
pub use cleanup::{CleanupAction, LoopExitKind, LoopUnwindPlan, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub use model::{
    Binding, BorrowRecord, ConditionalFact, DynamicRoleType, ExclusiveLoan, FatPointerLayout,
    GenericInstance, LiteralFact, Origin, OriginRecord, OriginRoot, ReachablePerformance,
    SemanticModel,
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
        crate::ast::MetaAttribute::Limitless(_) => true,
    })
}
pub(crate) use type_substitution::TypeSubstitution;
