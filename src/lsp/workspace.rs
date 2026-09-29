use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::lexer::scan;
use crate::parser::parse;

use super::documents::Document;

pub(super) const MAX_OPEN_DOCUMENTS: usize = 256;
pub(super) const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub(super) const MAX_OVERLAY_BYTES: usize = 16 * 1024 * 1024;
const MAX_MODULE_SIBLINGS: usize = 512;
const MAX_INDEXED_IMPORTS: usize = 1024;
const MAX_INDEXED_SYMBOLS: usize = 8192;

#[derive(Clone, Debug, Default)]
pub(super) struct WorkspaceModel {
    modules: BTreeMap<PathBuf, ModuleIndex>,
    reverse_dependencies: BTreeMap<String, BTreeSet<PathBuf>>,
    invalidated: BTreeSet<PathBuf>,
    generation: u64,
    overlay_bytes: usize,
    target: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ModuleIndex {
    pub(super) module_path: String,
    pub(super) facade: Option<PathBuf>,
    pub(super) siblings: Vec<PathBuf>,
    pub(super) imports: Vec<String>,
    pub(super) symbols: Vec<String>,
    pub(super) partial: bool,
    pub(super) version: i64,
}

impl WorkspaceModel {
    pub(super) fn open(
        &mut self,
        uri: &str,
        document_count: usize,
        text: &str,
    ) -> Result<(), String> {
        validate_overlay(document_count, self.overlay_bytes, 0, text.len())?;
        self.invalidate_dependents(uri_to_path(uri)?.as_path());
        self.overlay_bytes += text.len();
        self.generation += 1;
        self.invalidated.insert(uri_to_path(uri)?);
        Ok(())
    }

    pub(super) fn replace(
        &mut self,
        uri: &str,
        old_length: usize,
        new_length: usize,
    ) -> Result<(), String> {
        validate_overlay(0, self.overlay_bytes, old_length, new_length)?;
        self.invalidate_dependents(uri_to_path(uri)?.as_path());
        self.overlay_bytes = self.overlay_bytes - old_length + new_length;
        self.generation += 1;
        self.invalidated.insert(uri_to_path(uri)?);
        Ok(())
    }

    pub(super) fn close(&mut self, uri: &str, old_length: usize) {
        self.overlay_bytes = self.overlay_bytes.saturating_sub(old_length);
        self.generation += 1;
        if let Ok(path) = uri_to_path(uri) {
            self.invalidate_dependents(&path);
            self.remove_module(&path);
            self.invalidated.insert(path);
        }
    }

    pub(super) fn set_target(&mut self, target: &str) {
        if self.target.as_deref() != Some(target) {
            self.target = Some(target.to_owned());
            self.generation += 1;
            self.invalidated.extend(self.modules.keys().cloned());
        }
    }

    pub(super) fn refresh(&mut self, documents: &BTreeMap<String, Document>) {
        let paths = self.invalidated.clone();
        for path in paths {
            if let Some((uri, document)) = document_for_path(documents, &path) {
                self.reindex(&path, uri, document);
            } else {
                self.remove_module(&path);
            }
            self.invalidated.remove(&path);
        }
        self.overlay_bytes = documents.values().map(|document| document.text.len()).sum();
    }

    pub(super) fn overlays(
        &self,
        documents: &BTreeMap<String, Document>,
    ) -> HashMap<PathBuf, String> {
        documents
            .iter()
            .filter_map(|(uri, document)| {
                uri_to_path(uri).ok().map(|path| (path, document.text.clone()))
            })
            .collect()
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    fn reindex(&mut self, path: &Path, uri: &str, document: &Document) {
        self.remove_module(path);
        let Some(index) = index_document(path, uri, document) else { return };
        for import in &index.imports {
            self.reverse_dependencies.entry(import.clone()).or_default().insert(path.to_owned());
        }
        self.modules.insert(path.to_owned(), index);
    }

    fn invalidate_dependents(&mut self, path: &Path) {
        let Some(module_path) = self.modules.get(path).map(|index| index.module_path.clone())
        else {
            self.invalidated.insert(path.to_owned());
            return;
        };
        let mut pending = vec![module_path];
        while let Some(module) = pending.pop() {
            let dependents = self.reverse_dependencies.get(&module).cloned().unwrap_or_default();
            for dependent in dependents {
                if self.invalidated.insert(dependent.clone())
                    && let Some(index) = self.modules.get(&dependent)
                {
                    pending.push(index.module_path.clone());
                }
            }
        }
        self.invalidated.insert(path.to_owned());
    }

    fn remove_module(&mut self, path: &Path) {
        let Some(index) = self.modules.remove(path) else { return };
        for import in index.imports {
            if let Some(dependents) = self.reverse_dependencies.get_mut(&import) {
                dependents.remove(path);
                if dependents.is_empty() {
                    self.reverse_dependencies.remove(&import);
                }
            }
        }
    }
}

fn validate_overlay(
    document_count: usize,
    current_bytes: usize,
    removed_bytes: usize,
    added_bytes: usize,
) -> Result<(), String> {
    if document_count > MAX_OPEN_DOCUMENTS {
        return Err(format!("workspace has more than {MAX_OPEN_DOCUMENTS} open documents"));
    }
    if added_bytes > MAX_DOCUMENT_BYTES {
        return Err(format!("document exceeds {MAX_DOCUMENT_BYTES} byte limit"));
    }
    let total = current_bytes.saturating_sub(removed_bytes).saturating_add(added_bytes);
    if total > MAX_OVERLAY_BYTES {
        return Err(format!("workspace overlays exceed {MAX_OVERLAY_BYTES} byte limit"));
    }
    Ok(())
}

fn index_document(path: &Path, _uri: &str, document: &Document) -> Option<ModuleIndex> {
    let configuration = CompilerConfiguration::from_input_path_read_only(path).ok()?;
    let module_path = module_path(path, configuration.source_root())?;
    let (tokens, errors) = scan(&document.text);
    if !errors.is_empty() {
        return Some(empty_index(module_path, path, document.version));
    }
    let program = parse(tokens).ok()?;
    let (imports, symbols, partial) = declarations(&program);
    let (facade, siblings) = module_files(path, configuration.source_root());
    Some(ModuleIndex {
        module_path,
        facade,
        siblings,
        imports,
        symbols,
        partial,
        version: document.version,
    })
}

fn empty_index(module_path: String, path: &Path, version: i64) -> ModuleIndex {
    ModuleIndex {
        module_path,
        facade: Some(path.to_owned()),
        siblings: Vec::new(),
        imports: Vec::new(),
        symbols: Vec::new(),
        partial: true,
        version,
    }
}

fn declarations(program: &Program) -> (Vec<String>, Vec<String>, bool) {
    let mut imports = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Import(import) => {
                imports.insert(import.path.clone());
            }
            declaration => {
                if let Some(name) = declaration_name(declaration) {
                    symbols.insert(name.to_owned());
                }
            }
        }
    }
    let import_count = imports.len();
    let symbol_count = symbols.len();
    (
        imports.into_iter().take(MAX_INDEXED_IMPORTS).collect(),
        symbols.into_iter().take(MAX_INDEXED_SYMBOLS).collect(),
        import_count > MAX_INDEXED_IMPORTS || symbol_count > MAX_INDEXED_SYMBOLS,
    )
}

fn declaration_name(declaration: &TopLevelDecl) -> Option<&str> {
    match declaration {
        TopLevelDecl::Verb(value) => Some(&value.name),
        TopLevelDecl::ExternalVerb(value) => Some(&value.name),
        TopLevelDecl::Struct(value) => Some(&value.name),
        TopLevelDecl::Enum(value) => Some(&value.name),
        TopLevelDecl::Role(value) => Some(&value.name),
        TopLevelDecl::Pack(value) => Some(&value.name),
        _ => None,
    }
}

fn module_path(path: &Path, source_root: &Path) -> Option<String> {
    let relative = path.strip_prefix(source_root).ok()?;
    let stem = relative.file_stem()?.to_str()?;
    let parent = relative.parent()?;
    let parent_name = parent.file_name()?.to_str()?;
    if parent_name == stem || parent.join(format!("{parent_name}.act")).is_file() {
        return parent.to_str().map(|value| value.replace(['/', '\\'], "::"));
    }
    Some(stem.to_owned())
}

fn module_files(path: &Path, source_root: &Path) -> (Option<PathBuf>, Vec<PathBuf>) {
    let Some(parent) = path.parent() else { return (None, Vec::new()) };
    let name = parent.file_name().and_then(|value| value.to_str());
    let facade =
        name.map(|value| parent.join(format!("{value}.act"))).filter(|file| file.is_file());
    let mut siblings = fs::read_dir(parent)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok).map(|entry| entry.path()))
        .filter(|candidate| candidate.extension().and_then(|ext| ext.to_str()) == Some("act"))
        .filter(|candidate| Some(candidate) != facade.as_ref())
        .collect::<Vec<_>>();
    if parent == source_root {
        siblings.clear();
    }
    siblings.sort();
    siblings.truncate(MAX_MODULE_SIBLINGS);
    (facade, siblings)
}

fn document_for_path<'a>(
    documents: &'a BTreeMap<String, Document>,
    path: &Path,
) -> Option<(&'a str, &'a Document)> {
    documents.iter().find_map(|(uri, document)| {
        (uri_to_path(uri).ok().as_deref() == Some(path)).then_some((uri.as_str(), document))
    })
}

fn uri_to_path(uri: &str) -> Result<PathBuf, String> {
    let path =
        uri.strip_prefix("file://").ok_or_else(|| format!("unsupported document URI `{uri}`"))?;
    #[cfg(windows)]
    {
        Ok(PathBuf::from(path.trim_start_matches('/').replace('/', "\\")))
    }
    #[cfg(not(windows))]
    {
        Ok(PathBuf::from(path))
    }
}
