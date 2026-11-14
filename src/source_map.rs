use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::diagnostics::{Diagnostic, DiagnosticRelatedLocation};
use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Default)]
pub(crate) struct SourceMap {
    files: Vec<SourceFile>,
    next_offset: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceFile {
    path: PathBuf,
    identity: PathBuf,
    source: String,
    base: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceLocation {
    pub(crate) path: PathBuf,
    pub(crate) source: String,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug)]
pub(crate) struct SharedSourceMap(Arc<Mutex<SourceMap>>);

impl PartialEq for SharedSourceMap {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SharedSourceMap {}

impl SharedSourceMap {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(SourceMap::default())))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, SourceMap> {
        self.0.lock().expect("source map lock poisoned")
    }
}

impl SourceMap {
    pub(crate) fn register(&mut self, path: impl Into<PathBuf>, source: &str) -> usize {
        let path = path.into();
        let identity = canonical_path(&path);
        // A changed source for the same canonical path is a new revision with
        // its own base, so spans produced from earlier revisions stay valid.
        if let Some(file) =
            self.files.iter().find(|file| file.identity == identity && file.source == source)
        {
            return file.base;
        }
        let base = self.next_offset.max(1);
        self.next_offset = base.saturating_add(source.len()).saturating_add(1);
        self.files.push(SourceFile { path, identity, source: source.to_owned(), base });
        base
    }

    pub(crate) fn localize(
        &self,
        diagnostic: &Diagnostic,
    ) -> Option<(PathBuf, String, Diagnostic)> {
        let primary = self.location(diagnostic.span())?;
        let related = diagnostic
            .related_locations()
            .iter()
            .map(|location| {
                if let Some(mapped) = self.location(location.span()) {
                    DiagnosticRelatedLocation::new(
                        Some(mapped.path.display().to_string()),
                        mapped.span,
                        location.message(),
                    )
                } else {
                    location.clone()
                }
            })
            .collect();
        let path = primary.path.clone();
        let source = primary.source.clone();
        let localized = diagnostic.clone().with_localized_locations(
            Some(path.display().to_string()),
            primary.span,
            related,
        );
        Some((path, source, localized))
    }

    pub(crate) fn location(&self, span: SourceSpan) -> Option<SourceLocation> {
        self.files.iter().rev().find_map(|file| {
            let local_start = span.start.checked_sub(file.base)?;
            if local_start > file.source.len() {
                return None;
            }
            let local_end = span.end.saturating_sub(file.base).min(file.source.len());
            Some(SourceLocation {
                path: file.path.clone(),
                source: file.source.clone(),
                span: SourceSpan::new(local_start, local_end.max(local_start)),
            })
        })
    }
}

pub(crate) fn canonical_path(path: &std::path::Path) -> PathBuf {
    let path = path.to_owned();
    if let Ok(path) = std::fs::canonicalize(&path) {
        return path;
    }
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map_or(path.clone(), |directory| directory.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::SourceMap;
    use crate::lexer::SourceSpan;

    #[test]
    fn separates_eof_from_the_next_file() {
        let mut map = SourceMap::default();
        let first = map.register("source-map-first.act", "ab");
        let second = map.register("source-map-second.act", "cd");

        assert_eq!(second, first + 3);
        assert_eq!(map.location(SourceSpan::new(first + 2, first + 2)).unwrap().source, "ab");
        assert_eq!(map.location(SourceSpan::new(second, second)).unwrap().source, "cd");
    }

    #[test]
    fn canonical_relative_and_absolute_paths_share_a_base() {
        let mut map = SourceMap::default();
        let relative = std::path::PathBuf::from("source-map-canonical.act");
        let absolute = std::env::current_dir().unwrap().join(&relative);

        let first = map.register(&relative, "source");
        let second = map.register(absolute, "source");

        assert_eq!(first, second);
    }

    #[test]
    fn localizes_primary_and_related_locations() {
        let mut map = SourceMap::default();
        let first = map.register("source-map-related-first.act", "first");
        let second = map.register("source-map-related-second.act", "second");
        let diagnostic = crate::diagnostics::Diagnostic::error(
            "E1106",
            SourceSpan::new(second, second + 1),
            "duplicate declaration",
        )
        .with_related_location(crate::diagnostics::DiagnosticRelatedLocation::new(
            None,
            SourceSpan::new(first, first + 1),
            "first declaration",
        ));

        let (path, source, localized) = map.localize(&diagnostic).unwrap();
        assert!(path.ends_with("source-map-related-second.act"));
        assert_eq!(source, "second");
        assert_eq!(localized.span(), SourceSpan::new(0, 1));
        assert!(
            localized.related_locations()[0]
                .source_path()
                .unwrap()
                .ends_with("source-map-related-first.act")
        );
        assert_eq!(localized.related_locations()[0].span(), SourceSpan::new(0, 1));
    }
}
