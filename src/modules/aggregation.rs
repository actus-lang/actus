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
    UnknownSiblingModule { module: String, sibling: String, facade: PathBuf },
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
    for (source_index, source_path) in resolved.source_files().enumerate() {
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
        if source_index == 0 {
            validate_open_siblings(module_path, resolved.facade(), resolved.siblings(), &program)?;
        }
        for declaration in program.declarations {
            check_declaration_name(&declaration, source_path, &mut locations)?;
            declarations.push(declaration);
        }
    }
    Ok(Program { declarations })
}

pub fn exports_module(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<ModuleExports, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let mut parsed = Vec::new();
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
        parsed.push((source_path.to_owned(), program));
    }
    validate_open_siblings(module_path, resolved.facade(), resolved.siblings(), &parsed[0].1)?;
    Ok(collect_exports(&parsed))
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
        TopLevelDecl::OpenSibling(_) => None,
    }
}

fn validate_open_siblings(
    module_path: &str,
    facade: &Path,
    siblings: &[PathBuf],
    program: &Program,
) -> Result<(), ModuleError> {
    let known = siblings
        .iter()
        .filter_map(|path| path.file_stem().and_then(|stem| stem.to_str()))
        .collect::<std::collections::HashSet<_>>();
    for declaration in &program.declarations {
        if let TopLevelDecl::OpenSibling(sibling) = declaration
            && !known.contains(sibling.name.as_str())
        {
            return Err(ModuleError::UnknownSiblingModule {
                module: module_path.to_owned(),
                sibling: sibling.name.clone(),
                facade: facade.to_owned(),
            });
        }
    }
    Ok(())
}

fn collect_exports(parsed: &[(PathBuf, Program)]) -> ModuleExports {
    let facade_open = parsed[0]
        .1
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::OpenSibling(sibling) => Some(sibling.name.as_str()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let mut symbols = Vec::new();
    for (index, (path, program)) in parsed.iter().enumerate() {
        let sibling_name = path.file_stem().and_then(|stem| stem.to_str());
        if index == 0 || sibling_name.is_some_and(|name| facade_open.contains(name)) {
            symbols.extend(program.declarations.iter().filter_map(|declaration| {
                let (kind, name, _) = declaration_identity(declaration)?;
                declaration_is_open(declaration).then(|| ExportedSymbol {
                    kind: kind.to_owned(),
                    name: name.to_owned(),
                    source: path.clone(),
                })
            }));
        }
    }
    ModuleExports { symbols }
}

fn declaration_is_open(declaration: &TopLevelDecl) -> bool {
    match declaration {
        TopLevelDecl::Verb(value) => value.is_open,
        TopLevelDecl::ExternalVerb(value) => value.is_open,
        TopLevelDecl::Struct(value) => value.is_open,
        TopLevelDecl::Enum(value) => value.is_open,
        TopLevelDecl::Role(value) => value.is_open,
        TopLevelDecl::Perform(value) => value.is_open,
        TopLevelDecl::OpenSibling(_) => false,
    }
}
