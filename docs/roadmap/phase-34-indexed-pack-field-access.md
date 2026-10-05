# Phase 34: Indexed Pack-Field Access

**Status: Implementation in progress**

Phase 34 adds a safe, layout-aware way to access repeated fields inside a
packed value by index. The motivating form is:

```act
fabric.columns[source_idx].axons[index]
```

where `axons` is a repeated packed field with a fixed element type and a
compile-time element count. The feature must make repeated fixed-slot code
readable without changing the packed byte layout, ownership model, native ABI,
or bounds guarantees.

This phase is a language and compiler capability. It must not contain AIE,
robotics, tokenizer, or other application-specific vocabulary. Any example
used in compiler tests must be generic and reusable.

## Goals

- Represent repeated packed fields as one indexed field declaration.
- Preserve an explicit byte offset, element width, element count, and packing
  order for every indexed field.
- Support indexed reads and writes through normal Actus ownership and borrowing
  rules.
- Produce the same native address calculation as explicit fixed-field access.
- Provide deterministic bounds diagnostics for constant and runtime indexes.
- Keep named legacy field access available during migration where compatibility
  requires it.
- Make the feature readable in the formatter, formatter round trips, LSP
  metadata, hover, and definition lookup.

## Non-goals

- No dynamic allocation, slices, reflection, or runtime field discovery.
- No unchecked pointer arithmetic or raw operating-system pointers.
- No implicit conversion between an indexed packed field and a dynamic array.
- No hidden ownership transfer, mutation, cleanup, or bounds clamping.
- No application-specific aliases or compiler lowering for a single project.
- No change to the existing pack byte order, alignment, or serialization ABI.
- No replacement of bounded loops with an unbounded iterator abstraction.

## Proposed source contract

The final spelling must be confirmed by the design gate. The initial candidate is:

```act
pack Example {
    erg axons: Array[u32, 8] at 192;
    layout little;
}
```

The field declaration means eight contiguous `u32` elements beginning at bit
offset `192`. The valid index range is `0u32 .. 8u32`, and the byte address for
an element is derived from the declared base offset and element width. The
compiler must reject an element type or count that cannot be represented in the
pack's declared bounds.

The indexed expression must work for both immutable and mutable access:

```act
abs value: u32 = example.axons[index];
example.axons[index] = next_value;
```

The exact role syntax for indexed places must follow the existing Actus role
contract. A runtime index must remain explicitly typed and must be checked by
the generated bounds path; a compile-time constant index may be diagnosed at
compile time.

## Gates

### Gate 34.1 — Language and layout contract

- [x] Decide the canonical declaration syntax for repeated packed fields.
- [x] Define the relationship between base bit offset, element width, count,
      alignment, and total pack size.
- [x] Define little-endian and any future byte-order behavior.
- [x] Define whether indexed fields may coexist with named aliases for the same
      byte range.
- [x] Define compile-time constant-index diagnostics and runtime-index behavior.
- [x] Define malformed, overlapping, overflowing, and misaligned declaration
      diagnostics.
- [x] Record the accepted contract in an ADR before implementation.

### Gate 34.2 — Parser, AST, formatter, and LSP

- [x] Use the existing structured `TypeName` AST representation to preserve the field name,
      element type, count, base offset, and source spans.
- [x] Parse the declaration without treating it as a dynamic collection.
- [x] Parse indexed places for reads and writes.
- [x] Format declarations and expressions canonically.
- [x] Add formatter round-trip tests.
- [x] Expose element type, count, layout offset, and bounds in LSP hover and
      definition metadata.
- [x] Add deterministic diagnostics for malformed indexed-field syntax.

### Gate 34.3 — Semantic ownership and bounds analysis

- [x] Resolve indexed reads as immutable or mutable places according to the
      surrounding role contract.
- [x] Preserve `abs`, `ins`, `erg`, and `dat` semantics for the containing pack.
- [x] Reject indexed mutation through immutable views.
- [x] Reject indexes with incompatible integer types.
- [x] Reject provably out-of-range constant indexes.
- [x] Emit a checked runtime bounds path for non-constant indexes.
- [x] Verify cleanup and branch-join behavior for indexed place expressions.
- [x] Reject hidden conversion to a dynamic array, slice, or pointer.

### Gate 34.4 — Native lowering and layout parity

- [x] Lower indexed access to integer address arithmetic derived only from the
      declared pack layout.
- [x] Verify that element addresses use the declared element width and endian
      rules.
- [x] Preserve exact pack size and alignment metadata.
- [x] Compare indexed access with equivalent explicit named-field access where
      both representations are available.
- [x] Verify object and executable parity.
- [x] Verify that native lowering emits no floating-point instructions,
      allocation, or raw pointer escape.
- [x] Add diagnostics for unsupported target representations instead of falling
      back to an unsafe lowering.

### Gate 34.5 — Compatibility and migration

- [x] Define a migration path from repeated named fields to indexed fields.
- [x] Preserve source compatibility for named fields until the documented
      deprecation boundary.
- [x] Define whether aliases are generated, explicitly declared, or forbidden.
- [x] Verify that serialized bytes remain identical before and after migration.
- [x] Verify that public facade reachability and external ABI declarations remain
      unchanged.
- [x] Document formatter and LSP migration behavior.

### Gate 34.6 — Canonical public sub-facades

- [x] Treat canonical sub-facades inside a package facade as public API
      boundaries when they are opened by the parent facade.
- [x] Propagate only declarations marked `open` through every canonical facade
      layer, including `aie -> persistence -> shard` style chains.
- [x] Keep private declarations unavailable through the parent facade and
      reject direct sibling bypasses.
- [x] Accepted semantic and native regression tests cover public declarations
      reached through multiple nested facade layers.
- [x] Rejected visibility tests prove that unexported nested declarations
      cannot be called through the package facade.
- [x] Preserve the existing simple `import aie;` consumer syntax; named-import
      syntax is not required for this gate.

Evidence: the compiler facade suites and the full `cargo test --all --locked`
suite pass. The installed release compiler was rebuilt from this branch and
verified against Twin-e's `aie_persistence_scale` executable build; the build
completed in strict mode and native IR contained no floating-point
instructions. Public operations belong in the canonical sub-facade; private
implementation helpers remain behind that boundary.

### Gate 34.7 — Acceptance evidence

- [x] Add accepted tests for indexed reads and writes on packed fields.
- [x] Add accepted tests for nested indexed access through arrays of packs.
- [x] Add rejected tests for out-of-range constants, invalid runtime index
      types, immutable mutation, overlap, overflow, and misalignment.
- [x] Add layout-size, alignment, endian, and byte-offset tests.
- [x] Add ownership, cleanup, branch, and loop regression tests.
- [x] Add object/native parity tests and a no-floating-point IR audit.
- [x] Add formatter and LSP regression tests.
- [x] Record the compiler revision, test command, and native evidence.

Evidence: compiler revision `2a8dfce` contains the completed native/tooling
implementation. The implementation and acceptance tests are generic and do
not depend on an application package. Named scalar fields remain source
compatible, indexed fields do not generate aliases, and the ADR documents the
explicit migration and byte-layout compatibility policy.

## Exit criteria

Phase 34 is complete: every gate is checked, the ADR and compiler
contract are published, migration behavior is documented, and the full parser,
semantic, formatter, LSP, native, and regression suites pass. A readable source
example alone does not close this phase; the indexed access must be a verified
Actus language feature with preserved packed layout and ownership semantics.
