# ADR-0109: Compilation Session and Memoized Frontend Queries

## Status

Proposed

## Context

The compiler reparses the same canonical source files whenever a compilation
plan, a test file, or a module export lookup reaches them through another
path. One `actus test` invocation scans and parses each standard-library file
hundreds of times although its content does not change. This is frontend
work, not code-generation output, and it should be shared for the lifetime of
one compiler run.

ADR-0108 also requires declaration resolution to be cached for one
compilation session. Both needs call for one owner of per-run frontend state
instead of independent caches attached to unrelated structures.

## Decision

Introduce `CompilationSession` as the owner of frontend state for one compiler
run. CLI `check`, `build`, and `run` create one session for their complete
operation, and one `actus test` invocation shares one session across all test
files and collection workers.

The session owns:

- the ADR-0107 `SourceMap`, which keeps its single responsibility of source
  provenance and diagnostic localization;
- memoized frontend queries built on one generic `QueryCache`.

The first query is:

```text
parse(unit) -> Arc<ParsedSource>
```

`ParsedSource` holds the source text, the tokens, and exactly one outcome:
the program, the lexical errors, or the parse error. Callers receive an owned
program because normalization and annotation modify their copy.

### Keys and revisions

The query key is the ADR-0107 canonical source identity. A cached result is
returned only when its stored source is byte-identical to the requested
source; no hash stands in for the comparison. A changed source for the same
canonical path is a new revision: it replaces the published result for later
callers and is registered in the `SourceMap` with its own base offset, so spans
from earlier revisions remain valid. One source reached through two aliases
has one entry because both aliases share the canonical identity.

### Determinism

A hit returns the same tokens, AST, source spans, and diagnostics as a fresh
parse of the same revision. Base offsets depend on registration order, which
may vary between parallel workers; rendered diagnostics do not, because every
diagnostic is localized through the `SourceMap`.

### Concurrency

`QueryCache` holds its map lock only to look up or replace a slot. The
computation runs outside that lock: different keys are computed in parallel,
and concurrent callers of one key wait for a single computation. No caller
observes a partially published result. If a concurrent caller has published a
different revision of the same key, the current caller computes its own
revision without replacing the published one.

### Boundaries

Plan construction (`src/modules`, CLI `check`, `build`, and test collection)
scans and parses only through the session; an architecture conformance test
enforces this. Standalone tools keep explicit entry points that parse one
source without a session: the formatter, the AST dump command, and planless
module parsing used by the language server. The session does not own
code-generation or object output and does not create a persistent on-disk
cache.

### Follow-up integration

- PR B adds the ADR-0108 declaration registry and annotated module units as
  session queries on the same `QueryCache`.
- The language server currently parses through its planless entry point with
  overlays. A workspace session with document-version invalidation is a
  separate roadmap item (Phase 53, item L).

## Consequences

- Repeated frontend work within one run is removed without changing language
  semantics.
- Cache hits and fresh parses produce identical tokens, ASTs, spans, and
  diagnostics.
- Source provenance remains governed by ADR-0107.
- Memory is retained for one compiler run only; one entry is kept per
  canonical source.
- Test collection workers share one session and keep their parallelism.

## Compatibility

Compatible. This is a frontend performance and ownership change. Diagnostics,
spans, source resolution, and generated output are unchanged. No serialized
format, ABI, or code-generation cache changes.

## Alternatives

### Cache inside `SourceMap`

Rejected because it mixes provenance with frontend computation and leaves no
owner for later queries such as declaration resolution.

### Content hash as the cache key

Rejected because a hash collision would silently return another file's AST.
The stored source is compared byte for byte instead.

### Persistent disk cache

Rejected for this ADR because invalidation, portability, and stale diagnostic
handling require a separate design.

### A global process cache

Rejected because results could cross compilation runs and ownership of the
source map and document versions would become ambiguous.

### Per-call parsing

Rejected because it keeps the repeated work and prevents one canonical source
from sharing a result.

## References

- [ADR-0107: Unified Compilation-Plan Validation and Source Provenance](ADR-0107-unified-compilation-plan-validation-and-source-provenance.md)
- [ADR-0108: Canonical Declaration Identity](ADR-0108-canonical-declaration-identity.md)
- [Phase 53 alpha-release readiness](../roadmap/phase-53-alpha-release-readiness.md)
