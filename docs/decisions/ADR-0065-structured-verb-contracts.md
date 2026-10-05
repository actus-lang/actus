# ADR-0065: Structured Multi-line Verb Contracts

- Status: Accepted and implemented
- Date: 2026-10-05
- Decision owners: Actus language and compiler maintainers

## Context

Actus already supports triple-quoted documentation strings attached to
declarations. A long verb contract currently remains an unstructured block of
text, so tools cannot reliably show its purpose, ownership expectations,
failure behavior, or ABI notes as separate information. Adding executable
preconditions at the same time would mix documentation, semantic ownership,
and runtime behavior into one feature.

Phase 33.6 needs a readable contract form for public and internal verbs while
preserving the existing `""" ... """` documentation syntax and keeping the
compiler pipeline one-directional.

## Decision

Structured verb contracts are documentation metadata embedded in a verb's
leading triple-quoted documentation block. The block becomes structured only
when its first non-empty line is exactly `contract:`. Ordinary documentation
strings remain ordinary documentation strings and keep their current behavior.

The accepted section names are:

- `purpose`
- `inputs`
- `outputs`
- `ownership`
- `invariants`
- `errors`
- `side_effects`
- `abi`

Each section uses a lower-case name followed by `:` and owns the indented text
until the next section or the end of the documentation block:

```act
"""
contract:
purpose:
    Read one frame from the caller-owned input.
inputs:
    source: immutable bytes containing one complete frame.
outputs:
    Returns the decoded frame or a typed decoding error.
ownership:
    The `abs` view does not escape and the caller keeps ownership.
invariants:
    The declared frame length must fit inside the input buffer.
errors:
    Returns `DecodeError` for short, corrupt, or unsupported input.
side_effects:
    Does not allocate, mutate input, or write to the filesystem.
abi:
    Native lowering remains allocation-free and preserves the declared roles.
"""
open verb read_frame(abs source: Buffer) -> Result[Frame, DecodeError] {
    ...
}
```

The `contract:` marker is not a new executable statement and does not change
the verb signature. A section is text, not Actus code. Section content may
contain Markdown, backticks, punctuation, and multiple paragraphs. The parser
normalizes the same indentation already defined for docstrings and preserves
the section text for formatter and language-server consumers.

## Semantics and boundaries

- Contracts are documentation-only metadata in Phase 33.6.
- Contracts do not add runtime checks, alter ownership state, or affect code
  generation.
- The existing semantic analyzer remains responsible for real ownership,
  type, borrow, and return contracts.
- Executable preconditions and postconditions require a later ADR and a
  separate language feature.
- A contract is attached to the verb declaration span and is exported with an
  `open` verb through the existing facade visibility rules.
- Generic specialization preserves the contract metadata without duplicating
  or rewriting its text.
- External verbs may carry the same metadata, but the ABI section remains
  descriptive and cannot override the declared ABI signature.

## Parsing and diagnostics

The parser recognizes only the exact section names above. Unknown sections,
duplicate sections, content before the `purpose:` section, missing section
colons, and empty section names are rejected with stable parser diagnostics.
An ordinary docstring without `contract:` is never rejected as a malformed
contract. This keeps existing documentation source-compatible.

The AST stores a `VerbContract` separately from the raw normalized `doc` text.
The raw text remains available so documentation extraction and formatting do
not need to reconstruct the original block. Contract source ranges are tied
to the declaration and individual sections retain their normalized text.

## Tooling

The formatter emits the marker and sections in canonical section order while
preserving section paragraphs. LSP hover and documentation extraction expose
the structured sections when present and continue to expose ordinary doc text
for unstructured blocks. Completion and definition views may use the same
compiler-owned contract model; they must not infer sections by reparsing raw
source independently.

## Compatibility

Existing docstrings, verb signatures, generated native code, ABI lowering,
facade exports, and runtime behavior remain unchanged unless a source block
uses the new explicit `contract:` marker. The initial implementation is
documentation-only and adds no allocation or native symbol.

## Implementation gates

- [x] Define the section vocabulary and documentation-only boundary.
- [x] Define visibility and generic-specialization behavior.
- [x] Define malformed-contract rejection rules.
- [x] Add AST and parser representation.
- [x] Add formatter and LSP exposure for semantic model, hover, completion, signature help, and definition views.
- [x] Add complete facade, generic-specialization, definition-view, and round-trip acceptance coverage.
- [x] Update implementation guide for the shipped syntax and diagnostics.
