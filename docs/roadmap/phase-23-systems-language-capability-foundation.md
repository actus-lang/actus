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

- [ ] Add minimal positive and negative fixtures for each reported gap.
- [ ] Record the current diagnostic, source span, compiler stage, and expected
      behavior for every fixture.
- [ ] Identify the responsible lexer, parser, AST, semantic, codegen,
      formatter, LSP, and test modules before implementation.
- [ ] Confirm the solution remains target-neutral and does not add an
      application-specific AIE lowering path.
- [ ] Confirm all new implementation files remain below the repository hard
      size limit and are split by responsibility.
- [ ] Add a capability evidence index naming each test and command that closes
      a gate.

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

Nested `if`, `case`, and loop branches must produce a consistent semantic and
native control-flow graph.

- [ ] Support nested `if` statements inside expression branches and nested
      `if` expressions inside statements.
- [ ] Support `case` branches that return a typed value or diverge.
- [ ] Unify branch values only when their types are compatible.
- [ ] Treat `return`, `break`, `continue`, and typed `?` propagation as
      diverging/unwinding paths where appropriate.
- [ ] Preserve branch-local moves and borrows across joins.
- [ ] Emit deterministic cleanup on every normal and early edge.
- [ ] Lower merge values through explicit native block parameters or an
      equivalent backend-owned representation.
- [ ] Ensure indexed places and nested calls survive branch lowering without
      duplicated evaluation.
- [ ] Add repeated native execution tests for identical results and cleanup.

Representative expression:

```act
erg selected: u32 = if ready {
    if preferred { 41u32 } else { 42u32 }
} else {
    return 0;
};
```

## Gate 23.6: Formatter, LSP, and diagnostics parity

The language feature is incomplete until all compiler-owned tools understand
the same syntax and semantic model.

- [ ] Format const-generic declarations without changing their meaning.
- [ ] Format Boolean literals, nested places, and nested calls idempotently.
- [ ] Preserve Actus triple-quoted documentation strings and imports.
- [ ] Add LSP diagnostics for all rejected forms with correct spans.
- [ ] Add hover/type information for const parameters and Boolean literals.
- [ ] Add definition/navigation for nested fields, indexed bindings, and
      generic declarations.
- [ ] Ensure semantic tokens and completion do not treat `true`, `false`, or
      `Usize` as ordinary identifiers.
- [ ] Test open-document overlays and malformed/incomplete nested edits.
- [ ] Verify formatter and LSP do not move or delete source documentation.

## Gate 23.7: Strict and native acceptance

- [ ] Add a focused capability package under the repository's accepted example
      or integration-test location; do not add a separate target-specific
      language dialect.
- [ ] Make `actus check --strict` pass for the package.
- [ ] Make `actus test --strict` pass with accepted and rejected fixtures.
- [ ] Build an executable and verify native behavior for Boolean values,
      const-generic layout, nested indexed writes, and nested calls.
- [ ] Emit and inspect an object artifact where the ABI/layout contract is
      relevant.
- [ ] Verify no hidden allocation, C bridge, or handwritten lowering is used
      solely to make the fixtures pass.
- [ ] Run the repository quality suite and source-limit checks.
- [ ] Record exact commands, compiler revision, target profile, and expected
      outputs in the evidence index.

## Gate 23.8: Readiness for advanced systems workloads

- [ ] Demonstrate a bounded, non-application-specific fixture that combines
      const generics, fixed-width arithmetic, Boolean state, nested places,
      nested calls, typed errors, and deterministic cleanup.
- [ ] Demonstrate a fixed-size graph/fabric-like data structure without
      hardcoding its implementation into the compiler.
- [ ] Demonstrate serialization/snapshot code using ordinary Actus `Array`,
      `Buffer`, `pack`, and ownership contracts.
- [ ] Demonstrate bounded decay/threshold logic with typed literals and
      compound assignment.
- [ ] Verify the same source is understood by check, build, test, formatter,
      and LSP.
- [ ] Only after this gate is closed may an advanced systems workload be
      claimed as compiler-validated rather than source-level experimental code.

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

Phase 23 closes only when every gate has direct evidence, the three baseline
gaps are resolved, `actus check --strict` and `actus test --strict` pass for the
capability fixture, native execution is verified, and formatter/LSP behavior
matches the compiler. Any remaining language limitation must be recorded with
an owner, an exact unblock condition, and a dedicated roadmap item.
