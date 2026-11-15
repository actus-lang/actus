use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::queries::QueryCache;
use super::{CompilationSession, FrontendError, parse_source};

#[test]
fn parse_hit_returns_the_published_result() {
    let session = CompilationSession::new();
    let first = session.parse("session-hit.act", "verb main() { }");
    let second = session.parse("session-hit.act", "verb main() { }");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(session.parsed_entries(), 1);
}

#[test]
fn cached_and_uncached_parse_are_identical() {
    let session = CompilationSession::new();
    let source = "verb main() { erg value: Int = 1; }";
    let cached = session.parse("session-identical.act", source);
    let base = session.source_map().lock().register("session-identical.act", source);
    let fresh = super::parsed::ParsedSource::from_source(source, base);
    assert_eq!(*cached, fresh);
}

#[test]
fn canonical_aliases_share_one_parse() {
    let session = CompilationSession::new();
    let relative = std::path::PathBuf::from("session-alias.act");
    let absolute = std::env::current_dir().unwrap().join(&relative);
    let first = session.parse(&relative, "verb main() { }");
    let second = session.parse(absolute, "verb main() { }");
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(session.parsed_entries(), 1);
}

#[test]
fn changed_source_is_a_new_revision() {
    let session = CompilationSession::new();
    let first = session.parse("session-revision.act", "verb main() { }");
    let second = session.parse("session-revision.act", "verb changed() { }");
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(second.source(), "verb changed() { }");
    assert_eq!(session.parsed_entries(), 1);
    let third = session.parse("session-revision.act", "verb changed() { }");
    assert!(Arc::ptr_eq(&second, &third));
}

#[test]
fn frontend_failures_are_memoized_with_their_errors() {
    let session = CompilationSession::new();
    let lexed = session.parse("session-lex.act", "verb main() { $ }");
    assert!(matches!(lexed.program(), Err(FrontendError::Lex(_))));
    let parsed = session.parse("session-parse.act", "verb main( {");
    assert!(matches!(parsed.program(), Err(FrontendError::Parse(_))));
}

#[test]
fn declaration_and_annotation_queries_share_revisions() {
    let session = CompilationSession::new();
    let first = session.unit_declarations("std::io@revision-1", Default::default);
    let second = session.unit_declarations("std::io@revision-1", Default::default);
    assert!(Arc::ptr_eq(&first, &second));

    let program = || super::AnnotatedUnit {
        implementation: parse_source(
            None,
            std::path::Path::new("annotated.act"),
            "verb main() { }",
        )
        .program()
        .expect("test program parses"),
        sources: Vec::new(),
    };
    let annotated = session.annotated_unit("std::io@revision-1#std", program);
    let repeated = session.annotated_unit("std::io@revision-1#std", program);
    assert!(Arc::ptr_eq(&annotated, &repeated));

    let changed = session.annotated_unit("std::io@revision-2#std", program);
    assert!(!Arc::ptr_eq(&annotated, &changed));
}

#[test]
fn planless_parse_uses_file_relative_spans() {
    let parsed = parse_source(None, std::path::Path::new("planless.act"), "verb main() { }");
    assert_eq!(parsed.tokens()[0].span.start, 0);
}

#[test]
fn concurrent_callers_of_one_key_compute_once() {
    let cache = QueryCache::<u32, usize>::new();
    let computations = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                cache.get_or_compute(
                    &1,
                    |_| true,
                    || {
                        computations.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        7
                    },
                )
            });
        }
    });
    assert_eq!(computations.load(Ordering::SeqCst), 1);
}

#[test]
fn different_keys_compute_in_parallel() {
    let cache = QueryCache::<u32, bool>::new();
    let running = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for key in 0..2 {
            let (cache, running) = (&cache, &running);
            scope.spawn(move || {
                let overlapped = cache.get_or_compute(
                    &key,
                    |_| true,
                    || {
                        running.fetch_add(1, Ordering::SeqCst);
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(5);
                        while running.load(Ordering::SeqCst) < 2
                            && std::time::Instant::now() < deadline
                        {
                            std::thread::yield_now();
                        }
                        running.load(Ordering::SeqCst) == 2
                    },
                );
                // Both computations ran at the same time: a lock held around
                // computation would serialize them and exhaust the deadline.
                assert!(overlapped);
            });
        }
    });
    assert_eq!(cache.len(), 2);
}
