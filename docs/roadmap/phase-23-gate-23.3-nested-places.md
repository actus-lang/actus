# Gate 23.3 Evidence: Nested Places and Indexed Assignment

Status: **closed**. All implementation, semantic/native evidence, and required
repository quality checks pass.

## Scope

Gate 23.3 makes writable locations explicit in the compiler AST. It covers
simple bindings, field selectors, indexed selectors, nested selector chains,
compound assignments, ownership validation, and native address evaluation.
Nested call parsing and typed branch joins remain Gates 23.4 and 23.5.

## Current inventory

The parser already consumes selector chains such as
`fabric.columns[source_idx].axon_0` as nested expression nodes. Assignment
conversion currently has three statement variants (`Assignment`,
`FieldAssignment`, and `IndexAssignment`) and a separate recursive
`CompoundAssignmentTarget`. This duplicates place classification across the
parser, semantic analyzer, formatter, generic specialization, and native
lowering.

That split is the architectural gap for this gate. It makes the source shape
parseable, but does not provide one place contract that all compiler stages
can consume or one explicit boundary for evaluating a base and its selectors
exactly once.

## Planned implementation boundary

1. Introduce one AST `Place` representation for binding, field, and index
   selectors, including the complete source span for every node.
2. Convert assignment and compound-assignment statements to carry `Place`.
3. Centralize place-to-expression traversal only where a read view is needed;
   mutation and ownership checks must operate on `Place` directly.
4. Reuse the existing array, buffer, struct, pack, and loan backends behind a
   place lowering boundary that computes each base address once.
5. Add positive and negative parser, semantic, formatter, and native tests for
   arrays of structs, structs containing arrays, packs, and generic aggregates.

## Baseline evidence

The parser inventory test proves that the unified `Place` AST accepts nested
field/index chains and preserves their nested spans. It is backed by semantic
and native evidence rather than being treated as parser-only acceptance.

The current native evidence covers array-of-struct writes, nested compound
writes, nested `ins` loans, generic aggregates inside bounded arrays, and
read-only `abs` rejection. The generic aggregate case is validated through the
same place and field-specialization path as ordinary aggregates; it does not
use a place-specific workaround.

The direct indexed compound-assignment test and the nested array-of-struct
compound-assignment test cover the single-address lowering contract. The
lowering path computes the target address before the load, applies the
operation, and stores through that same address.

## Acceptance boundary

Gate 23.3 closes only when the unified place representation is used by all
assignment paths, invalid targets are rejected before codegen, ownership and
loan rules remain enforced, and native tests prove single address evaluation
for direct and compound nested writes. These conditions are now represented by
the implementation and tests listed above; the remaining closure decision is
the full repository quality gate.
