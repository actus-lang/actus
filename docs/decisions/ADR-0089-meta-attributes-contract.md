# ADR-0089: Meta-attribute contract and extension model

- Status: Proposed
- Date: 2026-10-28
- Scope: Actus metadata syntax, semantic validation, tooling, test runner,
  target selection, and source conformance

## Context

Actus already has several compiler-facing metadata attributes with different
consumers and different safety boundaries: test discovery, target filtering,
controlled test faults, and source-limit exceptions. Treating them as an
unstructured collection makes placement, argument validation, diagnostics, and
future extension easy to get wrong.

Metadata is not a runtime annotation system. Each attribute must have an
explicit compiler contract and must not silently alter ownership, ABI, binary
layout, or runtime safety.

## Decision

Every metadata attribute has a registered name, argument grammar, valid
attachment point, semantic effect, diagnostic contract, formatter behavior,
tooling behavior, and regression evidence. Unknown names and unsupported
arguments are rejected at compile time.

The current registry is:

| Attribute | Consumer | Contract |
| --- | --- | --- |
| `meta test` | test runner | discovers a test verb; does not make private code public |
| `meta target("...")` | target filtering | selects `unix`, `posix`, or `windows` declarations |
| `meta fault("...")` | strict test runner/runtime | enables one named test-only fault |
| `meta limitless("...")` | conformance policy | scopes a reviewed source-limit exception to a verb or file |

Metadata is processed before code generation. It may control discovery or
selection, but it cannot bypass parsing, semantic analysis, ownership checks,
ABI validation, or runtime safety rules.

## Required contract for future attributes

A proposed attribute must define:

1. its syntax and argument types;
2. valid declaration and file-prelude placement;
3. duplicate and ordering rules;
4. semantic and code-generation effects;
5. whether it is allowed in production builds or only in tests;
6. stable diagnostics for invalid use;
7. formatter and LSP behavior;
8. positive, negative, and cross-target fixtures;
9. compatibility and migration impact;
10. security, ownership, ABI, and performance consequences.

No attribute is accepted merely because the parser can store its name.

## Invariants

- Metadata has no implicit runtime representation.
- Test-only metadata cannot enter normal production behavior.
- `meta fault(...)` must be paired with `meta test` on the same declaration.
- Test metadata is valid only on an ordinary verb; external ABI declarations
  are not test-runner entry points.
- Target metadata cannot hide an unsupported target contract.
- Fault injection is disabled outside its explicit test configuration.
- Limit exceptions never disable language or runtime safety validation.
- Metadata diagnostics are deterministic and available to CLI and LSP clients.
- Formatter output preserves metadata placement and arguments.

## Implementation order

1. Keep the current attributes behaviorally compatible while documenting their
   contracts.
2. Centralize metadata validation and diagnostics without changing semantics.
3. Add formatter, LSP, and test-runner parity fixtures.
4. Add a new attribute only after its ADR amendment and mirrored fixtures are
   accepted.

## Consequences

The language gains a clear extension boundary and users can understand what
each attribute does without reading compiler internals. The cost is that a new
attribute requires coordinated parser, semantic, tooling, documentation, and
regression work instead of a parser-only change.

