use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::{LexError, SourceSpan, scan};
use crate::parser::{ParseError, parse};
use crate::semantic::{SemanticError, SemanticModel, analyze};

use super::resolver::{ModuleResolutionError, ModuleResolver};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleLocation {
    pub path: PathBuf,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum ModuleError {
    Resolution(ModuleResolutionError),
    Read { path: PathBuf, message: String },
    Lex { path: PathBuf, errors: Vec<LexError> },
    Parse { path: PathBuf, error: ParseError },
    DuplicateDeclaration(Box<DuplicateDeclaration>),
    Semantic(Box<SemanticError>),
}

#[derive(Debug)]
pub struct DuplicateDeclaration {
    pub kind: String,
    pub name: String,
    pub first: ModuleLocation,
    pub second: ModuleLocation,
}

impl Display for ModuleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolution(error) => Display::fmt(error, formatter),
            Self::Read { path, message } => {
                write!(formatter, "cannot read module source `{}`: {message}", path.display())
            }
            Self::Lex { path, errors } => write!(
                formatter,
                "cannot lex module source `{}` ({} errors)",
                path.display(),
                errors.len()
            ),
            Self::Parse { path, .. } => {
                write!(formatter, "cannot parse module source `{}`", path.display())
            }
            Self::DuplicateDeclaration(diagnostic) => write!(
                formatter,
                "duplicate {} declaration `{}` in `{}` and `{}`",
                diagnostic.kind,
                diagnostic.name,
                diagnostic.first.path.display(),
                diagnostic.second.path.display()
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

pub fn parse_module(resolver: &ModuleResolver, module_path: &str) -> Result<Program, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let mut declarations = Vec::new();
    let mut locations = HashMap::new();
    for source_path in resolved.source_files() {
        let source = fs::read_to_string(source_path).map_err(|error| ModuleError::Read {
            path: source_path.to_owned(),
            message: error.to_string(),
        })?;
        let (tokens, errors) = scan(&source);
        if !errors.is_empty() {
            return Err(ModuleError::Lex { path: source_path.to_owned(), errors });
        }
        let program = parse(tokens)
            .map_err(|error| ModuleError::Parse { path: source_path.to_owned(), error })?;
        for declaration in program.declarations {
            check_declaration_name(&declaration, source_path, &mut locations)?;
            declarations.push(declaration);
        }
    }
    Ok(Program { declarations })
}

pub fn analyze_module(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<SemanticModel, ModuleError> {
    let program = parse_module(resolver, module_path)?;
    analyze(&program).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

fn check_declaration_name(
    declaration: &TopLevelDecl,
    path: &Path,
    locations: &mut HashMap<(String, String), ModuleLocation>,
) -> Result<(), ModuleError> {
    let Some((kind, name, span)) = declaration_identity(declaration) else { return Ok(()) };
    let key = (kind.to_owned(), name.to_owned());
    let location = ModuleLocation { path: path.to_owned(), span };
    if let Some(first) = locations.insert(key, location.clone()) {
        return Err(ModuleError::DuplicateDeclaration(Box::new(DuplicateDeclaration {
            kind: kind.to_owned(),
            name: name.to_owned(),
            first,
            second: location,
        })));
    }
    Ok(())
}

fn declaration_identity(declaration: &TopLevelDecl) -> Option<(&'static str, &String, SourceSpan)> {
    match declaration {
        TopLevelDecl::Struct(value) => Some(("struct", &value.name, value.span)),
        TopLevelDecl::Enum(value) => Some(("enum", &value.name, value.span)),
        TopLevelDecl::Role(value) => Some(("role", &value.name, value.span)),
        TopLevelDecl::Verb(value) => Some(("verb", &value.name, value.span)),
        TopLevelDecl::ExternalVerb(value) => Some(("verb", &value.name, value.span)),
        TopLevelDecl::Perform(_) => None,
    }
}
