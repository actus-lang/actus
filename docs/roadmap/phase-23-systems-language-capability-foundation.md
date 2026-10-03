# Phase 23: Systems Language Capability Foundation

This phase closes compiler capabilities that are required before Actus can
reliably express large low-level systems workloads such as bounded protocol
engines, embedded kernels, register-oriented runtimes, and discrete inference
components. It is a language/compiler phase only. It does not add an AIE
application, a protocol library, a target-specific CPU dialect, or a source-
level workaround for a missing compiler feature.

The phase exists because a program can be syntactically plausible while still
being impossible to validate through the complete Actus pipeline. Every item
below must be proven through lexer/parser, AST, semantic, native-lowering,
formatter, LSP, and integration evidence where the item crosses those layers.

## Current baseline and known gaps

The current implementation has the following reported compiler gaps that block
production-shaped systems code:

1. Const-generic declarations such as `struct Fabric[N: Usize]` are rejected
   because the generic parameter grammar and type resolver do not yet accept
   a const parameter with a `Usize` domain. Gate 23.1 defines the first
   production slice as positive `Usize` literal arguments used in bounded
   `Array` capacities; const expressions and additional compile-time
   positions remain explicit follow-up work rather than implicit syntax.
2. Boolean literals exist in the lexical vocabulary but were not accepted as
   ordinary value expressions in every expression/initializer path. Gate 23.2
   closes this frontend, semantic, formatter, and native-lowering gap.
3. A nested-control-flow failure was reported for combinations of indexed and
   field places, nested calls, and value-producing branches. The minimal
   current fixture must reproduce the exact failure before parser changes are
   accepted. The reported shape includes:

   ```act
   if condition {
       fabric.columns[source_idx].axon_0 = slot;
       notify(value: slot);
   }
   ```

These statements are the baseline to reproduce with focused fixtures. They
must not be marked fixed from documentation or from a parser-only change.

The first reproduction results are recorded in
[Gate 23.0 baseline evidence](phase-23-gate-23.0-baseline.md). The current
minimal nested place/call fixture passes, so that report remains an open
minimization task rather than a confirmed compiler defect.

## Gate 23.0: Baseline reproduction and architecture contract

- [x] Add minimal positive and negative fixtures for each reported gap.
- [x] Record the current diagnostic, source span, compiler stage, and expected
      behavior for every fixture.
- [x] Identify the responsible lexer, parser, AST, semantic, codegen,
      formatter, LSP, and test modules before implementation.
- [x] Confirm the solution remains target-neutral and does not add an
      application-specific AIE lowering path.
- [x] Confirm all new implementation files remain below the repository hard
      size limit and are split by responsibility.
- [x] Add a capability evidence index naming each test and command that closes
      a gate.

Gate 23.0 is closed. The baseline, implementation ownership, and final
cross-layer evidence are recorded in the [baseline evidence](phase-23-gate-23.0-baseline.md)
and [capability evidence index](phase-23-capability-evidence-index.md).

## Gate 23.1: Const generic parameters

Const generics allow a type or verb to carry a compile-time numeric parameter
without turning that value into runtime mutable state.

Required declaration shape:

```act
struct CorticalFabric[N: Usize] {
    erg columns: Array[Minicolumn, N],
    erg active_count: Usize,
}
```

Required work:

- [x] Extend the AST generic-parameter model to distinguish type parameters
      from const parameters.
- [x] Define the accepted const-domain set, beginning with `Usize` and
      documenting whether additional fixed-width domains are allowed.
- [x] Parse `N: Usize` without treating `Usize` as an invalid integer type or
      an ordinary type-only bound.
- [x] Resolve const parameters in bounded array capacities. Pack/layout
      declarations, bounded arena capacities, and other positions remain
      outside the initial approved const-generic surface.
- [ ] Validate constant expressions, domain compatibility, non-negativity,
      representable capacity, and dependency cycles. The initial slice
      validates positive representable `Usize` literals; expression support is
      deferred until a constant-expression model is specified.
- [x] Preserve const arguments in generic identity, layout identity, and
      native specialization for the initial array-capacity slice. Cache
      identity and const expressions remain open.
- [x] Reject runtime expressions in const-generic positions before codegen.
- [x] Keep const-generic values out of mutable runtime storage unless source
      explicitly materializes a value.
- [x] Add diagnostics for missing arguments, wrong domains, invalid values,
      and non-const array capacities. Overflow, duplicate const parameters,
      and unresolved const references remain follow-up diagnostics.
- [x] Add accepted/rejected parser, semantic, layout, and native execution
      tests for the initial `Usize`/`Array` slice. Object-level identity and
      broader compile-time positions remain open.

Gate 23.1 initial-slice evidence is recorded in
[Gate 23.1 const-generic evidence](phase-23-gate-23.1-const-generics.md).

Acceptance example:

```act
struct Cell {
    erg charge: u8,
}

struct Fabric[N: Usize] {
    erg cells: Array[Cell, N],
}

verb make_fabric() -> Fabric[8] {
    return Fabric[8] { cells: Array[Cell, 8]() };
}
```

The exact aggregate initializer must match the accepted compiler grammar; the
fixture is a capability contract, not permission to bypass type checking.

## Gate 23.2: Boolean literal expressions

`true` and `false` must be first-class expressions everywhere an ordinary
literal may appear, subject to normal type checking.

- [x] Add boolean literal nodes to the AST expression model.
- [x] Accept boolean literals in local initializers, return values, call
      arguments, struct fields, enum payloads, array elements, constants,
      conditions, case expressions, and nested blocks.
- [x] Resolve both literals to `Bool` without numeric coercion.
- [x] Lower boolean values correctly in branches, merges, comparisons, and
      native returns.
- [x] Preserve literal spans in diagnostics, formatter output, hover, and
      semantic model responses.
- [x] Reject `true`/`false` where an integer, buffer, or unrelated aggregate is
      required, with a stable type diagnostic.
- [x] Prove short-circuit behavior when a Boolean literal controls `&&` or
      `||`.
- [x] Add hosted and freestanding/object-level evidence where applicable.
      Hosted native evidence is present; freestanding/object parity remains
      part of Gate 23.7.

Minimum acceptance fixtures:

```act
const DEFAULT_LINKED: Bool = false;

verb bool_values() -> Bool {
    erg linked: Bool = false;
    if linked {
        return false;
    }
    return true;
}
```

## Gate 23.3: Nested places and indexed assignment parsing

Status: **closed**. Gate 23.3 replaces the former
split assignment variants with one typed place representation and re-proves
semantic ownership and native address evaluation for complete selector chains.

An lvalue/place is a writable location, not merely a simple identifier. The
parser and AST must represent nested paths without losing evaluation order or
ownership information.

Required place forms include:

```act
binding = value;
object.field = value;
object.field[index] = value;
object[index].field = value;
object[index].field[index] = value;
```

Required work:

- [x] Define one AST place representation shared by simple, field, index, and
      nested compound-assignment targets.
- [x] Parse nested field/index chains in statement position and expression
      position where the language permits them.
- [x] Preserve source spans for the complete place and each selector.
- [x] Ensure the base expression is evaluated exactly once for assignment and
      compound assignment.
- [x] Reject indexing a non-indexable value, field access on a non-aggregate,
      and mutation through `abs` or suspended owners during semantic analysis.
- [x] Preserve `ins` loans for aggregate slots without copying the containing
      array or buffer.
- [x] Add parser and semantic tests for arrays of structs, structs containing
      arrays, packs, and nested generic aggregates.

The following must be a normal compiler path, not an application exception:

```act
fabric.columns[source_idx].axon_0 = slot;
```

The implementation and acceptance evidence are recorded in [Gate 23.3
nested-place evidence](phase-23-gate-23.3-nested-places.md). The full repository
quality checks pass for this gate.

## Gate 23.4: Nested call and statement parsing

Status: **closed**. Calls inside nested blocks are
parsed as statements when their result is
discarded and as expressions when their result is consumed.

- [x] Parse calls inside `if`, `else`, `case` block bodies, loops, and nested
      lexical blocks.
- [x] Parse named arguments and explicit ownership markers in every nested
      call position.
- [x] Preserve `?` propagation on nested fallible calls.
- [x] Distinguish a call statement from a declaration, assignment, or malformed
      expression without recovery ambiguity.
- [x] Verify nested calls with `erg`, `abs`, `dat`, and `ins` arguments.
- [x] Add diagnostics for missing semicolons, invalid ownership markers,
      unknown verbs, wrong argument names, and incompatible return use.
- [x] Prove that a nested call cannot bypass cleanup or borrow-state updates.

Acceptance shape:

```act
if condition {
    notify(value: slot);
    update(target: ins buffer)?;
}
```

The implementation and acceptance evidence are recorded in [Gate 23.4
nested-call evidence](phase-23-gate-23.4-nested-calls.md). The full repository
quality checks pass for this gate.

## Gate 23.5: Typed nested control-flow joins

Status: **closed**. Nested conditional and case branches now share typed
semantic joins and explicit native merge/termination edges. Diverging return
paths are emitted as native terminators instead of being mistaken for
fallthrough paths.

- [x] Support nested `if` statements inside expression branches and nested
      `if` expressions inside statements.
- [x] Support `case` branches that return a typed value or diverge.
- [x] Unify branch values only when their types are compatible.
- [x] Treat `return`, `break`, `continue`, and typed `?` propagation as
      diverging/unwinding paths where appropriate.
- [x] Preserve branch-local moves and borrows across joins.
- [x] Emit deterministic cleanup on every normal and early edge.
- [x] Lower merge values through explicit native block parameters or an
      equivalent backend-owned representation.
- [x] Ensure indexed places and nested calls survive branch lowering without
      duplicated evaluation.
- [x] Add repeated native execution tests for identical results and cleanup.

Representative expression:

```act
erg selected: u32 = if ready {
    if preferred { 41u32 } else { 42u32 }
} else {
    return 0;
};
```

The implementation and acceptance evidence are recorded in [Gate 23.5 typed
control-flow evidence](phase-23-gate-23.5-control-flow-joins.md). The full
repository quality checks pass for this gate.

## Gate 23.6: Formatter, LSP, and diagnostics parity

The language feature is incomplete until all compiler-owned tools understand
the same syntax and semantic model.

- [x] Format const-generic declarations without changing their meaning.
- [x] Format Boolean literals, nested places, and nested calls idempotently.
- [x] Preserve Actus triple-quoted documentation strings and imports.
- [x] Add LSP diagnostics for all rejected forms with correct spans.
- [x] Add hover/type information for const parameters and Boolean literals.
- [x] Add definition/navigation for nested fields, indexed bindings, and
      generic declarations.
- [x] Ensure semantic tokens and completion do not treat `true`, `false`, or
      `Usize` as ordinary identifiers.
- [x] Test open-document overlays and malformed/incomplete nested edits.
- [x] Verify formatter and LSP do not move or delete source documentation.

## Gate 23.7: Strict and native acceptance

- [x] Add a focused capability package under the repository's accepted example
      or integration-test location; do not add a separate target-specific
      language dialect.
- [x] Make `actus check --strict` pass for the package.
- [x] Make `actus test --strict` pass with accepted and rejected fixtures.
- [x] Build an executable and verify native behavior for Boolean values,
      const-generic layout, nested indexed writes, and nested calls.
- [x] Emit and inspect an object artifact where the ABI/layout contract is
      relevant.
- [x] Verify no hidden allocation, C bridge, or handwritten lowering is used
      solely to make the fixtures pass.
- [x] Run the repository quality suite and source-limit checks.
- [x] Record exact commands, compiler revision, target profile, and expected
      outputs in the evidence index.

## Gate 23.8: Readiness for advanced systems workloads

- [x] Demonstrate a bounded, non-application-specific fixture that combines
      const generics, fixed-width arithmetic, Boolean state, nested places,
      nested calls, typed errors, and deterministic cleanup.
- [x] Demonstrate a fixed-size graph/fabric-like data structure without
      hardcoding its implementation into the compiler.
- [x] Demonstrate serialization/snapshot code using ordinary Actus `Array`,
      `Buffer`, `pack`, and ownership contracts.
- [x] Demonstrate bounded decay/threshold logic with typed literals and
      compound assignment.
- [x] Verify the same source is understood by check, build, test, formatter,
      and LSP.
- [x] Only after this gate is closed may an advanced systems workload be
      claimed as compiler-validated rather than source-level experimental code.

## Phase 23 extension: multi-word packed storage

The original Phase 23 acceptance scope is closed, but the readiness workload
exposed a separate compiler capability required for cache-line-sized register
maps and fixed binary state: a `pack` currently accepts only one fixed-width
unsigned scalar as its storage representation. An array-backed storage field
such as:

```act
open pack Minicolumn {
    erg storage: Array[u8, 64];
    layout little;
    fields {
        erg charge: u8 at 0;
        erg threshold: u8 at 8;
        erg coincidence_low: u64 at 64;
        erg inhibitory_link: u32 at 480;
    }
}
```

must not be treated as a source-level workaround. It requires a complete
representation, semantic, native, tooling, and acceptance contract. The
following extension gates remain open and are the next Phase 23 work items.

### Gate 23.9: Multi-word pack representation contract

- [x] Define the AST and semantic representation for scalar and contiguous
      multi-word pack storage without leaking Cranelift or C ABI types into the
      frontend.
- [x] Accept a bounded storage form beginning with `Array[u8, N]` and define
      the supported element families, capacity domain, and maximum bit width.
- [x] Preserve the existing `pack` field syntax and explicit bit offsets;
      reject negative offsets, overlapping fields, offsets past storage width,
      and non-representable total layouts before code generation.
- [x] Define endianness for byte-array storage and document the mapping between
      byte index, bit offset, and field value.
- [x] Define deterministic size, alignment, stride, and zero-initialization
      rules for multi-word packs. The representation must be inline and
      bounded; it must not allocate a heap object.
- [x] Add accepted and rejected parser/AST/semantic fixtures, including the
      exact `Array[u8, 64]`/512-bit shape needed by the systems workload.

Gate 23.9 is closed for the frontend and semantic representation contract.
Native argument passing, inline code generation, indexed storage operations,
and executable serialization remain intentionally owned by Gates 23.11 and
23.12; no native support is claimed until those gates provide direct evidence.

### Gate 23.10: Multi-word layout and type-system validation

- [x] Register array-backed pack storage as a first-class semantic layout
      identity, distinct from an ordinary `Array[u8, N]` value.
- [x] Validate storage capacity and field offsets using checked compile-time
      arithmetic with deterministic diagnostics for overflow and width errors.
- [x] Preserve pack identity, storage width, endianness, field metadata,
      generic type keys, and defaults in the semantic layout contract. Native
      specialization keys remain owned by Gate 23.12.
- [x] Resolve the validated field model against one storage representation,
      preserving field width, signedness, defaults, reserved fields, and
      ownership roles for later native lowering.
- [x] Reject invalid storage element types, runtime capacities, unsupported
      nested storage, duplicate fields, overlapping fields, and out-of-range
      bit slices before native lowering.
- [x] Keep pack field metadata and raw-storage identity derived from the same
      AST storage contract; executable indexed access and aliasing proofs remain
      owned by Gate 23.11.
- [x] Add negative tests for invalid layouts and positive tests for exact
      64-byte/512-bit layouts, including field metadata and defaults.

Gate 23.10 is closed for the frontend and semantic layout boundary. It does
not claim native byte indexing, executable multi-word field access, or binary
serialization; those capabilities remain explicitly scoped to Gates 23.11 and
23.12.

### Gate 23.11: Indexed storage access and ownership semantics

- [x] Parse and type-check `column.storage[index]` for array-backed pack
      storage as a normal nested place and expression, including reads,
      writes, compound assignments, and calls from nested control flow.
- [x] Evaluate the base pack and index exactly once at the semantic place
      boundary; native address lowering remains covered by Gate 23.12.
- [x] Enforce bounds checks using the declared storage capacity and reject
      statically impossible indexes where the existing constant rules allow it.
- [x] Preserve `erg`, `abs`, `ins`, and `dat` contracts when storage bytes are
      borrowed, inspected, mutated, or transferred through the existing place
      and loan machinery. An `ins` loan restores the same pack owner.
- [x] Reject mutation through `abs` storage, overlapping loans, use after
      move/drop, and invalid storage aliases with stable diagnostics through
      the shared indexed-place validation path.
- [x] Add semantic tests for byte reads/writes, nested control flow, and
      mutation through an `abs` pack owner. Native execution and cleanup
      evidence that requires inline multi-word lowering remains Gate 23.12.

Gate 23.11 is closed for indexed-storage parsing, semantic bounds, and
ownership contracts. Native multi-word address calculation, executable byte
access, and serialization remain intentionally unclaimed until Gate 23.12.

### Gate 23.12: Native lowering, ABI, and serialization

- [x] Lower multi-word pack storage to contiguous inline native memory with
      deterministic alignment and no hidden allocation or C bridge.
- [x] Lower indexed storage operations through one checked address
      calculation, load/store width, and bounds path; never duplicate a
      side-effecting base or index expression.
- [x] Lower field access at the declared bit offset with correct masking,
      shifting, sign/zero extension, and endianness for every supported field
      width.
- [x] Define and test initialization, copy/move, drop, return, argument
      passing, and aggregate assignment for 64-byte packs.
- [x] Provide deterministic byte snapshot/restore behavior using ordinary
      Actus operations. Round trips must preserve every byte and every declared
      field value across supported endianness modes.
- [x] Inspect emitted objects and execute host-native tests for exact layout,
      indexed storage mutation, field mutation, bounds failure, and snapshot
      round trips.
- [x] Add a freestanding/object-level parity test where the target contract
      supports the representation; do not add a target-specific language path.

Gate 23.12 evidence: `tests/arrays_cli.rs` covers indexed storage and bounds
traps, bitfields, unaligned multi-byte fields, little- and big-endian access,
array-backed pack return slots, Buffer snapshot/restore, and deterministic
object emission. `cargo test --test arrays_cli` passes all 57 native/object
tests; repository source-limit and compiler quality checks are required before
the gate commit.

Gate 23.12 is closed for the current supported representation: array-backed
pack storage is lowered as an inline byte region, while scalar-storage packs
retain their existing scalar ABI. No target-specific syntax or C serialization
bridge is introduced.

### Gate 23.13: Formatter, LSP, and diagnostics parity for packed storage

- [x] Format array-backed pack declarations idempotently without changing
      storage type, field offsets, layout keywords, or documentation strings.
- [x] Add parser and semantic diagnostics with exact spans for invalid storage
      elements, capacities, offsets, overlap, width, and endianness.
- [x] Add LSP hover and semantic model data for storage width, byte capacity,
      field offsets, field widths, and endianness.
- [x] Add definition/navigation, completion, semantic-token, and rename
      coverage for `storage` indexes and pack fields.
- [x] Verify open-document overlays reindex pack layout changes and do not
      report stale `unknown type` or field metadata errors.
- [x] Test malformed nested storage expressions and invalid incremental edits
      without crashing or partially mutating the workspace overlay.

Gate 23.13 evidence: formatter coverage is provided by
`formats_array_backed_packs_idempotently_with_layout_metadata` and
`keeps_pack_storage_and_field_documentation_in_place` in `tests/formatter`.
Pack contract diagnostics retain stable E1070-E1074 classifications and source
spans through semantic tests in `tests/semantic/packs.rs`; the LSP projection
is pinned by `lsp_reports_pack_storage_contracts_with_stable_codes_and_spans`.
The LSP suite covers storage width/bytes/capacity/layout facts, hover,
completion, semantic tokens, overlay reindexing, storage definition, and rename
through `lsp_exposes_array_pack_layout_facts_and_reindexes_overlay_changes`,
`lsp_exposes_pack_registers_in_hover_completion_and_tokens`, and
`lsp_navigates_and_renames_array_pack_storage`. Existing workspace recovery
tests cover malformed nested overlays and invalid incremental edits without
partial mutation. The formatter and LSP suites pass with 18 and 75 tests,
respectively; repository quality checks are required before the gate commit.

### Gate 23.14: Packed-storage systems readiness acceptance

- [ ] Replace the current experimental `Minicolumn` storage blocker with a
      real accepted fixture using `Array[u8, 64]` and the complete 512-bit
      field map.
- [ ] Make `actus check --strict`, `actus test --strict`, `actus fmt --check`,
      native executable build/run, and object emission pass for the fixture.
- [ ] Prove indexed byte storage, bit-packed field access, fixed-width
      arithmetic, nested calls, Boolean control flow, ownership cleanup, and
      serialization in one bounded workload.
- [ ] Add rejected fixtures for invalid field overlap, out-of-range offsets,
      wrong storage element types, runtime capacity, bounds violations, and
      ownership violations.
- [ ] Verify the compiler, formatter, LSP, source-limit checks, documentation
      checks, and full repository quality suite are green.
- [ ] Update the Phase 23 evidence index with exact test names, commands,
      target profile, expected outputs, and the closing compiler revision.

No gate in this extension may be closed by changing the AIE source to use a
different representation. The compiler capability itself must be implemented,
tested, and documented first.

## Non-goals

This phase does not:

- implement an AIE, Wire, Ustari, operating system, or neural runtime;
- add CPU names or microcontroller families to the Actus language syntax;
- introduce implicit numeric conversions;
- weaken ownership, bounds, cleanup, or ABI validation;
- add a parser-only shortcut that leaves semantic or native behavior undefined;
- mark a workload complete because its source files exist;
- treat a passing `cargo check` as proof of Actus native execution.

## Completion criteria

The original Phase 23 scope is closed through Gate 23.8: the three baseline
gaps have direct evidence, the capability fixture passes strict check/test,
native execution is verified, and formatter/LSP behavior matches the
compiler. The multi-word packed-storage extension closes only when Gates
23.9–23.14 have direct evidence and the 64-byte systems fixture passes the
same strict, native, object, formatter, LSP, source-limit, documentation, and
repository quality checks. Any remaining language limitation must be recorded
with an owner, an exact unblock condition, and a dedicated roadmap item.
