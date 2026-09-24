mod aggregation;
mod resolver;

pub use aggregation::{
    DuplicateDeclaration, ModuleError, ModuleLocation, analyze_module, parse_module,
};
pub use resolver::{ModuleResolutionError, ModuleResolver, ResolvedModule};
