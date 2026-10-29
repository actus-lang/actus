# Phase 48: Meta-attribute contract and extension readiness

This phase establishes a stable, documented contract for Actus metadata before
new attributes are proposed. It does not introduce a new attribute by itself.

## Scope

- `meta test`
- `meta target(...)`
- `meta fault(...)`
- `meta limitless(...)`
- parser, semantic validation, formatter, LSP, test-runner, and diagnostics
  boundaries shared by those attributes

## Work plan

- [x] Inventory every existing metadata form, argument, placement rule, and
      consumer.
- [x] Link each attribute to a focused handbook document under
      `docs/guide/meta-attributes/`.
- [ ] Accept ADR-0089 after implementation and compatibility review.
- [x] Add the first parser negative fixture for test-only fault metadata.
- [ ] Add the remaining parser and semantic negative fixtures for malformed,
      duplicate, and misplaced metadata.
- [x] Add formatter and LSP fixtures that preserve metadata boundaries.
- [x] Verify test-only metadata is unavailable in ordinary production paths.
- [x] Verify target metadata on Unix, POSIX, and Windows target contracts.
- [ ] Verify every supported fault with a strict fixture and no invented names.
- [ ] Audit `limitless` exceptions against the source-limit policy.
- [ ] Record compiler version, platform matrix, strict checks, and unresolved
      limitations before adding new metadata ideas.

## Completion standard

The phase is complete only when the existing attributes have one consistent
documented contract, the relevant fixtures pass, diagnostics are stable, and
the implementation is ready for a separately reviewed proposal for any new
attribute.

