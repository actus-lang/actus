use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, sort_diagnostics};

#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "policy.rs"]
mod policy;
#[path = "scan.rs"]
mod scan;

use diagnostics::{file_diagnostics, function_diagnostics};
pub use policy::SourceLimitPolicy;
use scan::find_functions;

/// Inspects one source file and returns every source-limit diagnostic it violates.
pub fn inspect_source(path: &Path, source: &str) -> Vec<Diagnostic> {
    source_limit_diagnostics(path, source, SourceLimitPolicy::default())
}

/// Applies a source-limit policy to one Rust or Actus source file.
pub fn source_limit_diagnostics(
    path: &Path,
    source: &str,
    policy: SourceLimitPolicy,
) -> Vec<Diagnostic> {
    let mut diagnostics = file_diagnostics(path, source, source.lines().count(), policy);
    for function in find_functions(source) {
        diagnostics.extend(function_diagnostics(path, source, &function, policy));
    }
    diagnostics
}

/// Inspects all Rust and Actus source files below the supplied roots.
pub fn inspect_source_tree(roots: &[&Path]) -> io::Result<Vec<Diagnostic>> {
    let files = source_paths(roots)?;
    let mut diagnostics = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).map_err(|error| {
            io::Error::new(error.kind(), format!("cannot read `{}`: {error}", path.display()))
        })?;
        diagnostics.extend(inspect_source(&path, &source));
    }
    sort_diagnostics(&mut diagnostics);
    Ok(diagnostics)
}

/// Returns all Rust and Actus source paths below the supplied roots in
/// deterministic lexical order.
pub fn source_paths(roots: &[&Path]) -> io::Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    for root in roots {
        collect_source_files(root, &mut files)?;
    }
    Ok(files.into_iter().collect())
}

fn collect_source_files(path: &Path, files: &mut BTreeSet<PathBuf>) -> io::Result<()> {
    if path.is_file() {
        if is_source_file(path) {
            files.insert(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        collect_source_files(&entry?.path(), files)?;
    }
    Ok(())
}

fn is_source_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs" || extension == "act")
}
