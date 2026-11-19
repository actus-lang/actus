use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::SharedSourceMap;
use crate::ast::Program;
use crate::modules::aggregation::registry::DeclarationRegistry;
use crate::source_map::canonical_path;

use super::parsed::ParsedSource;
use super::queries::QueryCache;

/// One compiler run (ADR-0109): source provenance and memoized frontend
/// queries shared by every compilation in that run.
#[derive(Clone)]
pub(crate) struct CompilationSession {
    state: Arc<SessionState>,
}

struct SessionState {
    source_map: SharedSourceMap,
    parsed: QueryCache<PathBuf, Arc<ParsedSource>>,
    unit_declarations: QueryCache<String, Arc<DeclarationRegistry>>,
    annotated_units: QueryCache<String, Arc<AnnotatedUnit>>,
}

/// Annotated programs of one module unit: the merged implementation and each
/// source file, in source order (ADR-0108 annotation, ADR-0109 query).
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct AnnotatedUnit {
    pub(crate) implementation: Program,
    pub(crate) sources: Vec<Program>,
}

impl CompilationSession {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(SessionState {
                source_map: SharedSourceMap::new(),
                parsed: QueryCache::new(),
                unit_declarations: QueryCache::new(),
                annotated_units: QueryCache::new(),
            }),
        }
    }

    pub(crate) fn source_map(&self) -> &SharedSourceMap {
        &self.state.source_map
    }

    /// Memoizes the declaration view for one unit revision and import closure.
    pub(crate) fn unit_declarations(
        &self,
        key: impl Into<String>,
        compute: impl FnOnce() -> DeclarationRegistry,
    ) -> Arc<DeclarationRegistry> {
        self.state.unit_declarations.get_or_compute(&key.into(), |_| true, || Arc::new(compute()))
    }

    /// Memoizes one annotated unit for one unit revision and import closure.
    pub(crate) fn annotated_unit(
        &self,
        key: impl Into<String>,
        compute: impl FnOnce() -> AnnotatedUnit,
    ) -> Arc<AnnotatedUnit> {
        self.state.annotated_units.get_or_compute(&key.into(), |_| true, || Arc::new(compute()))
    }

    /// Memoized `parse(unit)`, keyed by canonical source identity. A hit
    /// requires byte-identical source; a changed source is a new revision.
    pub(crate) fn parse(&self, path: impl AsRef<Path>, source: &str) -> Arc<ParsedSource> {
        let path = path.as_ref();
        self.state.parsed.get_or_compute(
            &canonical_path(path),
            |parsed| parsed.source() == source,
            || {
                let base = self.state.source_map.lock().register(path, source);
                Arc::new(ParsedSource::from_source(source, base))
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn parsed_entries(&self) -> usize {
        self.state.parsed.len()
    }
}

/// Parses through `session` when one exists; planless tooling parses the
/// source on its own with spans relative to the file.
pub(crate) fn parse_source(
    session: Option<&CompilationSession>,
    path: &Path,
    source: &str,
) -> Arc<ParsedSource> {
    session.map_or_else(
        || Arc::new(ParsedSource::from_source(source, 0)),
        |session| session.parse(path, source),
    )
}

impl PartialEq for CompilationSession {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
}

impl Eq for CompilationSession {}

impl fmt::Debug for CompilationSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("CompilationSession").finish_non_exhaustive()
    }
}
