pub(crate) mod aggregation;
mod package_sources;
mod resolution_error;
mod resolver;

pub(crate) use aggregation::DeclarationRegistry;
pub use aggregation::{
    DuplicateDeclaration, ExportedSymbol, ModuleCompilationPlan, ModuleError, ModuleExports,
    ModuleIdentity, ModuleLocation, ModuleNamespace, ModuleObjectOwner, ModuleObjectPlan,
    ModuleObjectUnit, ModuleSource, ModuleSourceKind, ModuleUnit, OutOfScopeReference,
    ScopeViolation, analyze_module, analyze_module_unit, analyze_module_with_overlays,
    analyze_module_with_overlays_for_target, analyze_with_imports, analyze_with_imports_for_target,
    analyze_with_imports_with_overlays_for_target, build_compilation_plan,
    build_compilation_plan_with_overlays, build_test_compilation_plan, exports_module,
    exports_module_with_overlays, load_module_unit, load_module_unit_with_overlays, parse_module,
    resolve_imports,
};
pub(crate) use aggregation::{program_with_performance_context, validate_compilation_plan};
pub use package_sources::{PackageRole, PackageSourceIdentity};
pub use resolver::{ModuleResolutionError, ModuleResolver, ResolvedChildModule, ResolvedModule};
