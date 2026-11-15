mod analysis;
mod collision;
mod declarations;
mod exports;
mod identity;
mod objects;
mod parsing;
mod plan;
pub(crate) mod registry;
mod scope;
mod types;
mod unit;
mod validation;
mod visibility;

pub use analysis::{
    analyze_module, analyze_module_unit, analyze_module_with_overlays,
    analyze_module_with_overlays_for_target, analyze_with_imports, analyze_with_imports_for_target,
    analyze_with_imports_with_overlays_for_target, resolve_imports,
};
pub use exports::{exports_module, exports_module_with_overlays};
pub use identity::ModuleNamespace;
pub use objects::{ModuleObjectOwner, ModuleObjectPlan, ModuleObjectUnit};
pub use parsing::parse_module;
pub use plan::{
    ModuleCompilationPlan, build_compilation_plan, build_compilation_plan_with_overlays,
    build_test_compilation_plan,
};
pub(crate) use plan::{program_with_performance_context, validate_compilation_plan};
pub(crate) use registry::DeclarationRegistry;
pub use types::{
    DuplicateDeclaration, ExportedSymbol, ModuleError, ModuleExports, ModuleLocation,
    ScopeViolation,
};
pub use unit::{
    ModuleIdentity, ModuleSource, ModuleSourceKind, ModuleUnit, load_module_unit,
    load_module_unit_with_overlays,
};
