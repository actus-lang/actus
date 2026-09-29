mod analysis;
mod exports;
mod parsing;
mod types;
mod validation;

pub use analysis::{
    analyze_module, analyze_module_with_overlays, analyze_module_with_overlays_for_target,
    analyze_with_imports, analyze_with_imports_for_target, resolve_imports,
};
pub use exports::exports_module;
pub use parsing::parse_module;
pub use types::{DuplicateDeclaration, ExportedSymbol, ModuleError, ModuleExports, ModuleLocation};
