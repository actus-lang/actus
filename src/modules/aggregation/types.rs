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
    Semantic(Box<crate::semantic::SemanticError>),
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
            Self::PrivateDeclarationAccess { module, symbol, facade, .. } => write!(
                formatter,
                "private declaration `{symbol}` from module `{module}` is not exported by facade `{}`",
                facade.display()
            ),
            Self::Semantic(error) => write!(
                formatter,
                "module semantic error at {}..{}",
                error.span.start, error.span.end
            ),
        }
    }
}

impl std::error::Error for ModuleError {}

fn write_read_error(
    formatter: &mut Formatter<'_>,
    path: &PathBuf,
    message: &str,
) -> std::fmt::Result {
    write!(formatter, "cannot read module source `{}`: {message}", path.display())
}

fn write_lex_error(
    formatter: &mut Formatter<'_>,
    path: &PathBuf,
    count: usize,
) -> std::fmt::Result {
    write!(formatter, "cannot lex module source `{}` ({count} errors)", path.display())
}

fn write_parse_error(formatter: &mut Formatter<'_>, path: &PathBuf) -> std::fmt::Result {
    write!(formatter, "cannot parse module source `{}`", path.display())
}

fn write_import_binding_error(formatter: &mut Formatter<'_>, binding: &str) -> std::fmt::Result {
    write!(formatter, "import binding `{binding}` is declared more than once")
}
