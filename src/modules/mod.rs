mod aggregation;
mod resolver;

pub use aggregation::{
    DuplicateDeclaration, ExportedSymbol, ModuleCompilationPlan, ModuleError, ModuleExports,
    ModuleIdentity, ModuleLocation, ModuleSource, ModuleSourceKind, ModuleUnit, analyze_module,
    analyze_module_unit, analyze_module_with_overlays, analyze_module_with_overlays_for_target,
    analyze_with_imports, analyze_with_imports_for_target, build_compilation_plan, exports_module,
    load_module_unit, parse_module, resolve_imports,
};
pub use resolver::{ModuleResolutionError, ModuleResolver, ResolvedModule};
