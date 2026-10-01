use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ast::{LimitlessScope, MetaAttribute, TopLevelDecl};
use crate::diagnostics::{Diagnostic, sort_diagnostics};
use crate::lexer::SourceSpan;
use crate::lexer::scan;
use crate::parser::parse;

#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "policy.rs"]
mod policy;
#[path = "scan.rs"]
mod scan;

use diagnostics::{file_diagnostics, function_diagnostics, suppression_diagnostics};
pub use policy::SourceLimitPolicy;
use scan::find_functions;

/// Records why a source-limit exception was accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitlessApprovalOrigin {
    SourceMetadata,
    PackageConfiguration,
}

/// One reviewable source-limit exception accepted for a source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitlessApproval {
    pub path: PathBuf,
    pub scope: &'static str,
    pub target: Option<String>,
    pub span: SourceSpan,
    pub origin: LimitlessApprovalOrigin,
}

/// Conformance diagnostics together with the accepted exception evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLimitReport {
    pub diagnostics: Vec<Diagnostic>,
    pub approvals: Vec<LimitlessApproval>,
}

/// Inspects one source file and returns every source-limit diagnostic it violates.
pub fn inspect_source(path: &Path, source: &str) -> Vec<Diagnostic> {
    source_limit_diagnostics(path, source, SourceLimitPolicy::default())
}

/// Inspects one source file when project policy keeps source limits enabled.
pub fn inspect_source_with_enforcement(
    path: &Path,
    source: &str,
    enforce_limits: bool,
) -> Vec<Diagnostic> {
    if enforce_limits { inspect_source(path, source) } else { Vec::new() }
}

/// Inspects one source while recording whether acceptance came from source metadata or package policy.
pub fn inspect_source_report_with_enforcement(
    path: &Path,
    source: &str,
    enforce_limits: bool,
) -> SourceLimitReport {
    if enforce_limits {
        return inspect_source_report(
            path,
            source,
            SourceLimitPolicy::default(),
            LimitlessApprovalOrigin::SourceMetadata,
        );
    }
    SourceLimitReport {
        diagnostics: Vec::new(),
        approvals: vec![LimitlessApproval {
            path: fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
            scope: "file",
            target: None,
            span: SourceSpan::new(0, source.len()),
            origin: LimitlessApprovalOrigin::PackageConfiguration,
        }],
    }
}

/// Applies a source-limit policy to one Rust or Actus source file.
pub fn source_limit_diagnostics(
    path: &Path,
    source: &str,
    policy: SourceLimitPolicy,
) -> Vec<Diagnostic> {
    inspect_source_report(path, source, policy, LimitlessApprovalOrigin::SourceMetadata).diagnostics
}

/// Inspects one source and returns diagnostics plus accepted exception evidence.
pub fn inspect_source_report(
    path: &Path,
    source: &str,
    policy: SourceLimitPolicy,
    origin: LimitlessApprovalOrigin,
) -> SourceLimitReport {
    let exemptions = source_limit_exemptions(source);
    let mut diagnostics = if exemptions.file {
        Vec::new()
    } else {
        file_diagnostics(path, source, source.lines().count(), policy)
    };
    for function in find_functions(source) {
        if !exemptions.verbs.contains(&function.name) && !exemptions.file {
            diagnostics.extend(function_diagnostics(path, source, &function, policy));
        }
    }
    if !exemptions.file {
        diagnostics.extend(suppression_diagnostics(path, source));
    }
    let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let approvals = exemptions
        .approvals
        .into_iter()
        .map(|(scope, target, span)| LimitlessApproval {
            path: canonical_path.clone(),
            scope,
            target,
            span,
            origin,
        })
        .collect();
    SourceLimitReport { diagnostics, approvals }
}

#[derive(Default)]
struct SourceLimitExemptions {
    file: bool,
    verbs: HashSet<String>,
    approvals: Vec<(&'static str, Option<String>, SourceSpan)>,
}

fn source_limit_exemptions(source: &str) -> SourceLimitExemptions {
    let (tokens, errors) = scan(source);
    if !errors.is_empty() {
        return SourceLimitExemptions::default();
    }
    let Ok(program) = parse(tokens) else { return SourceLimitExemptions::default() };
    let mut exemptions = SourceLimitExemptions {
        file: program.file_metadata.contains(&LimitlessScope::File),
        ..SourceLimitExemptions::default()
    };
    if exemptions.file {
        exemptions.approvals.push(("file", None, SourceSpan::new(0, source.len())));
    }
    for declaration in program.declarations {
        let (name, span) = match declaration {
            TopLevelDecl::Verb(verb) => (
                verb.metadata
                    .iter()
                    .any(|attribute| {
                        matches!(attribute, MetaAttribute::Limitless(LimitlessScope::Verb))
                    })
                    .then_some(verb.name),
                verb.span,
            ),
            TopLevelDecl::ExternalVerb(verb) => (
                verb.metadata
                    .iter()
                    .any(|attribute| {
                        matches!(attribute, MetaAttribute::Limitless(LimitlessScope::Verb))
                    })
                    .then_some(verb.name),
                verb.span,
            ),
            _ => (None, SourceSpan::new(0, 0)),
        };
        if let Some(name) = name {
            exemptions.verbs.insert(name.clone());
            exemptions.approvals.push(("verb", Some(name), span));
        }
    }
    exemptions
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
