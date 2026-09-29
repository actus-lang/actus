# ADR-0042 Implementation Roadmap: Bounded Arrays and Native Pack Lowering

This roadmap implements the general systems primitives accepted by ADR-0042.
It does not define an application-specific data structure or execution model.
Each gate requires accepted and rejected tests before it can be marked
complete.

## Gate 0: Contract and representation baseline

- [x] Accept ADR-0042 as the governing systems-primitives decision.
- [x] Define the canonical source spelling for `Array[T, N]`.
- [x] Define element layout, alignment, capacity, and zero-length behavior.
- [x] Define the stable bounds-failure contract for hosted and freestanding
      targets.
- [x] Inventory existing pack layouts and native field access limitations.
- [x] Record the array and pack representation in the language and backend
      documentation.

## Gate 1: Lexer, parser, and AST

- [x] Add deterministic token support for bounded array type syntax.
- [x] Parse `Array[T, N]` type applications with a compile-time capacity.
- [x] Represent bounded arrays in the existing generic type AST with a
      dedicated capacity argument and source span.
- [x] Parse `expr[index]` indexing expressions with complete source spans.
- [x] Add AST representation for indexed reads and indexed assignments.
- [x] Preserve ownership-role annotations for indexed expressions where the
      grammar permits `abs` and `ins` access.
- [x] Reject malformed array capacities and incomplete index expressions with
      stable parser diagnostics.
- [x] Add lexer and parser tests for valid, malformed, nested, and ambiguous
      array/index syntax.

## Gate 2: Semantic validation and bounds checking

- [x] Validate array element types and compile-time capacities.
- [x] Validate index expressions as supported integer types.
- [x] Reject statically provable out-of-bounds constant indices.
- [x] Define runtime bounds-check insertion for dynamic indices.
- [x] Reject unsupported indexing targets and invalid assignment types.
- [x] Add stable diagnostics for capacity, index, and element-type failures.
- [x] Add positive and negative semantic fixtures for reads and writes.

## Gate 3: Ownership and slot loans

- [x] Define `abs array[index]` as a read-only, non-escaping slot view.
- [x] Define `ins array[index]` as an exclusive call-scope slot loan.
- [x] Preserve array provenance through slot views and nested calls.
- [x] Reject slot aliasing, escaping views, relocation, and drop while loaned.
- [x] Prove that slot loans do not copy the complete array.
- [x] Add ownership transition and cleanup tests for success and early exit.

## Gate 4: Cranelift native pack lowering

- [x] Lower packed-field reads using target-aware shift and mask operations.
- [x] Lower packed-field writes using clear-mask and insert operations.
- [x] Preserve unrelated backing bits during every write.
- [x] Implement signed extraction and sign extension by declared field type.
- [x] Validate field range before native insertion.
- [x] Add native tests for endianness, widths, overflow, and overlapping layout
      rejection.

## Gate 5: Cranelift native array lowering and bounds failures

- [x] Lower contiguous array layout for stack and arena storage.
- [x] Lower dynamic reads and writes without whole-array copies.
- [x] Emit deterministic target-specific bounds failures.
- [x] Lower indexed pack access through the same validated layout contract.
- [x] Add native tests for valid access, constant rejection, and dynamic failure.
- [x] Verify cleanup and arena provenance across indexed operations.

## Gate 6: Native execution and end-to-end quality verification

- [x] Execute arrays and indexed pack fields through native binaries.
- [x] Verify indexed `ins` slot mutation and caller reuse without allocation.
- [x] Verify deterministic artifacts and repeated-run results.
- [x] Add hosted and freestanding target evidence where supported.
- [x] Run formatter, check, clippy, tests, source limits, and diff checks.
- [x] Update ADR-0042 with implementation evidence and approved deviations.
- [x] Mark this roadmap complete after all diagnostics and native tests pass.
