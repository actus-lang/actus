# ADR-0107: Unified Compilation-Plan Validation and Source Provenance

## Status

Proposed

## Context

`check` currently analyzes only the caller program
(`src/cli/check.rs:136-163`), while strict build validates the caller and
each object unit (`src/cli/build/loading.rs:92-140`). Backend emission also
re-analyzes a performance-context program
(`src/codegen/shared_lowering/orchestration.rs:66-75`, `269-279`). This
allows the R1 and R2 reproductions to pass `check` and fail during build.

R1 is one enum made available through two `open` re-export paths; build
resolves the paths as two different types and reports E1027. R2 is a
const-generic struct obtained through a re-export that reaches build as
`UnknownTypeParameter`/E1052. The reproductions and evidence are recorded in
the [Phase 53 roadmap](../roadmap/phase-53-alpha-release-readiness.md).

Source spans currently contain only byte offsets
(`src/lexer/token.rs:1-12`). Consequently, a unit diagnostic can be reported
against the entry file instead of the source file that owns the span.

## Decision

Introduce one compilation-session `SourceMap` without changing `SourceSpan`
or the AST. Each loaded source receives a deterministic base offset, starting
at 1, in entry-then-resolver order. Scanner spans become global during a
compilation session; standalone `scan(source)` and LSP analysis retain local
offsets.

Introduce unified compilation-plan validation in
`src/modules/aggregation/plan/validation.rs`. It validates the caller, every
object unit, target filtering, and the performance-context program used by
orchestration. Check, build, run, and test use the validator according to
their existing mode.

After validation, semantic failure inside orchestration is an explicit
internal compiler error rather than raw `Debug` output.

## Consequences

- Build diagnostics identify the source file that owns the unit.
- Check and build reject the same invalid compilation plans.
- Spans are unique within a compilation session, which removes cross-file
  collisions in span-keyed semantic caches and synthesized names.
- Base offsets are deterministic, so repeated builds of the same input retain
  byte-identical output.
- LSP ranges remain document-local.
- Existing span tests and diagnostic snapshots require provenance-aware
  expectations.

## Compatibility

Compatible — bug fix. Programs that already build retain their behavior.
Programs that already fail during build may now be rejected earlier by
`check`, with the correct source path and diagnostic. Span-based internal
synthetic names such as `__try_payload_*` and `__nested_if_*` change when
their source offsets become session-global; the internal unit ABI is not
stable during alpha (`docs/release/versioning.md`).

## Alternatives

Adding `SourceFileId` to `SourceSpan` or the AST was rejected. It would change
the public span and AST contracts and propagate source identity through every
AST value. Source provenance belongs to the compilation session and renderer,
not to the language-level span type.
