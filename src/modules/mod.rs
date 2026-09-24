mod aggregation;
mod resolver;

pub use aggregation::{
    DuplicateDeclaration, ExportedSymbol, ModuleError, ModuleExports, ModuleLocation,
    analyze_module, exports_module, parse_module,
};
pub use resolver::{ModuleResolutionError, ModuleResolver, ResolvedModule};
