use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use crate::lexer::{LexError, SourceSpan};
use crate::parser::ParseError;

use super::super::resolver::ModuleResolutionError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleLocation {
    pub path: PathBuf,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum ModuleError {
    Resolution(ModuleResolutionError),
    UnknownSiblingModule {
        module: String,
        sibling: String,
        facade: PathBuf,
    },
    ConfigurationImport {
        module: String,
        path: PathBuf,
        span: SourceSpan,
    },
    ConfigurationRuntimeDeclaration {
        module: String,
        path: PathBuf,
        kind: String,
        span: SourceSpan,
    },
    Read {
        path: PathBuf,
        message: String,
    },
    Lex {
        path: PathBuf,
        errors: Vec<LexError>,
    },
    Parse {
        path: PathBuf,
        error: Box<ParseError>,
    },
    DuplicateDeclaration(Box<DuplicateDeclaration>),
    SymbolCollision {
        symbol: String,
        first_module: String,
        second_module: String,
    },
    ImportBindingCollision {
        binding: String,
        first: SourceSpan,
        second: SourceSpan,
    },
    DuplicateObjectOwner {
        module_path: String,
    },
    PrivateDeclarationAccess {
        module: String,
        symbol: String,
        facade: PathBuf,
        span: SourceSpan,
    },
    /// One facade opens a name both publicly and for its unit (ADR-0111).
    ConflictingOpening {
        module: String,
        name: String,
        facade: PathBuf,
        span: SourceSpan,
    },
    OutOfScopeReference {
        module: String,
        symbol: String,
        reason: ScopeViolation,
        path: PathBuf,
        span: SourceSpan,
    },
    Semantic(Box<crate::semantic::SemanticError>),
}

/// Why a spelling does not name a declaration in its module's scope
/// (ADR-0110).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScopeViolation {
    /// Declared by a module that is neither the referencing module, one of
    /// its ancestors, nor one of its opened children.
    NotInScope { declared_in: String },
    /// Declared by a child module that does not export it.
    PrivateToChild { declared_in: String },
    /// Brought in by an import of another module; imports are not inherited.
    ImportNotInherited { imported_by: String },
    /// Qualified by a binding of a module of the unit that does not declare it.
    NotDeclared { module: String },
    /// Reached through an import of another unit's child module that does
    /// not export it.
    NotExported { module: String },
}

#[derive(Debug)]
pub struct DuplicateDeclaration {
    pub kind: String,
    pub name: String,
    pub first: ModuleLocation,
    pub second: ModuleLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportedSymbol {
    pub kind: String,
    pub name: String,
    pub source: PathBuf,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleExports {
    pub symbols: Vec<ExportedSymbol>,
}

impl ModuleExports {
    pub fn contains(&self, kind: &str, name: &str) -> bool {
        self.symbols.iter().any(|symbol| symbol.kind == kind && symbol.name == name)
    }
}

impl Display for ModuleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolution(error) => Display::fmt(error, formatter),
            Self::UnknownSiblingModule { module, sibling, facade } => write!(
                formatter,
                "module `{module}` facade `{}` references unknown sibling `{sibling}.act`",
                facade.display()
            ),
            Self::ConfigurationImport { module, path, .. } => write!(
                formatter,
                "configuration module `{module}` cannot import `{}`",
                path.display()
            ),
            Self::ConfigurationRuntimeDeclaration { module, path, kind, .. } => write!(
                formatter,
                "configuration module `{module}` cannot contain runtime declaration `{kind}` in `{}`",
                path.display()
            ),
            Self::Read { path, message } => write_read_error(formatter, path, message),
            Self::Lex { path, errors } => write_lex_error(formatter, path, errors.len()),
            Self::Parse { path, .. } => write_parse_error(formatter, path),
            Self::DuplicateDeclaration(diagnostic) => write!(
                formatter,
                "duplicate {} declaration `{}` in `{}` and `{}`",
                diagnostic.kind,
                diagnostic.name,
                diagnostic.first.path.display(),
                diagnostic.second.path.display()
            ),
            Self::SymbolCollision { symbol, first_module, second_module } => write!(
                formatter,
                "native symbol `{symbol}` would collide between modules `{first_module}` and `{second_module}`"
            ),
            Self::ImportBindingCollision { binding, .. } => {
                write_import_binding_error(formatter, binding)
            }
            Self::DuplicateObjectOwner { module_path } => {
                write!(formatter, "duplicate object ownership for module `{module_path}`")
            }
            Self::PrivateDeclarationAccess { module, symbol, facade, .. } => {
                write_private_access(formatter, module, symbol, facade)
            }
            Self::ConflictingOpening { module, name, .. } => {
                write_conflict(formatter, module, name)
            }
            Self::OutOfScopeReference { module, symbol, reason, .. } => {
                write_scope_violation(formatter, module, symbol, reason)
            }
            Self::Semantic(error) => write!(
                formatter,
                "module semantic error at {}..{}",
                error.span.start, error.span.end
            ),
        }
    }
}

impl std::error::Error for ModuleError {}

impl ModuleError {
    pub(crate) fn frontend(path: &std::path::Path, error: crate::session::FrontendError) -> Self {
        match error {
            crate::session::FrontendError::Lex(errors) => {
                Self::Lex { path: path.to_owned(), errors }
            }
            crate::session::FrontendError::Parse(error) => {
                Self::Parse { path: path.to_owned(), error: Box::new(error) }
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn write_read_error(
    formatter: &mut Formatter<'_>,
    path: &PathBuf,
    message: &str,
) -> std::fmt::Result {
    write!(formatter, "cannot read module source `{}`: {message}", path.display())
}

#[allow(clippy::ptr_arg)]
fn write_lex_error(
    formatter: &mut Formatter<'_>,
    path: &PathBuf,
    count: usize,
) -> std::fmt::Result {
    write!(formatter, "cannot lex module source `{}` ({count} errors)", path.display())
}

#[allow(clippy::ptr_arg)]
fn write_parse_error(formatter: &mut Formatter<'_>, path: &PathBuf) -> std::fmt::Result {
    write!(formatter, "cannot parse module source `{}`", path.display())
}

fn write_import_binding_error(formatter: &mut Formatter<'_>, binding: &str) -> std::fmt::Result {
    write!(formatter, "import binding `{binding}` is declared more than once")
}

fn write_private_access(
    formatter: &mut Formatter<'_>,
    module: &str,
    symbol: &str,
    facade: &std::path::Path,
) -> std::fmt::Result {
    write!(
        formatter,
        "private declaration `{symbol}` from module `{module}` is not exported by facade `{}`",
        facade.display()
    )
}

fn write_conflict(formatter: &mut Formatter<'_>, module: &str, name: &str) -> std::fmt::Result {
    write!(
        formatter,
        "facade of module `{module}` opens `{name}` both with `open {name};` and `open unit {name};`; keep one"
    )
}

fn write_scope_violation(
    formatter: &mut Formatter<'_>,
    module: &str,
    symbol: &str,
    reason: &ScopeViolation,
) -> std::fmt::Result {
    match reason {
        ScopeViolation::NotInScope { declared_in } => write!(
            formatter,
            "`{symbol}` is declared in module `{declared_in}`, which is not in scope of module `{module}`; add `import {{ open {declared_in} }}` to a file of `{module}`"
        ),
        ScopeViolation::PrivateToChild { declared_in } => write!(
            formatter,
            "`{symbol}` is private to child module `{declared_in}`; declare it `open` and open its file in the `{declared_in}` facade to use it from `{module}`"
        ),
        ScopeViolation::NotDeclared { module: declaring } => write!(
            formatter,
            "`{symbol}` names no declaration of module `{declaring}` or its children; qualify it with the module that declares it in `{module}`"
        ),
        ScopeViolation::NotExported { module: child } => write!(
            formatter,
            "`{symbol}` is not exported by module `{child}`, which `{module}` imports; only the declarations its facade opens are reachable"
        ),
        ScopeViolation::ImportNotInherited { imported_by } => write!(
            formatter,
            "`{symbol}` is imported by module `{imported_by}`, not by `{module}`; imports are not inherited, so add the import to a file of `{module}`"
        ),
    }
}
