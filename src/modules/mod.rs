mod aggregation;
mod resolver;

pub use aggregation::{
    DuplicateDeclaration, ExportedSymbol, ModuleError, ModuleExports, ModuleLocation,
    analyze_module, analyze_module_with_overlays, analyze_with_imports, exports_module,
    parse_module, resolve_imports,
};
pub use resolver::{ModuleResolutionError, ModuleResolver, ResolvedModule};
