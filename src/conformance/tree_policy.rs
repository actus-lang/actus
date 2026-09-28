use std::io;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, DiagnosticPhase, STRICT_ACTUS_FIXTURE_RUST_SOURCE};

/// Finds Rust implementation files embedded in the Actus fixture tree.
pub fn fixture_tree_diagnostics(root: &Path) -> io::Result<Vec<Diagnostic>> {
    let fixture_root = root.join("tests/library");
    let mut paths = Vec::new();
    collect_files(&fixture_root, &mut paths)?;
    Ok(paths
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            Diagnostic::error(
                STRICT_ACTUS_FIXTURE_RUST_SOURCE,
                crate::lexer::SourceSpan::new(0, 0),
                "Rust source is forbidden in `tests/library/` fixtures",
            )
            .with_source_path(path.display().to_string())
            .with_phase(DiagnosticPhase::Conformance)
            .with_explanation(
                "Standard-library fixture behavior must be expressed as Actus source; Rust test logic belongs under `tests/`.",
            )
            .with_suggestion("Move Rust integration logic outside `tests/library/` and keep the fixture in Actus." )
        })
        .collect())
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    for entry in std::fs::read_dir(path)? {
        collect_files(&entry?.path(), files)?;
    }
    files.sort();
    Ok(())
}
