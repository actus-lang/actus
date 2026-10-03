# Phase 23 Gate 23.0: Baseline Evidence

This document records the first reproducible baseline for Phase 23. It is
deliberately evidence-oriented: a reported compiler limitation is not treated
as confirmed until a minimal source fixture reproduces it in the current
checkout.

## Environment

- Repository: `/home/magradze/Projects/actus_project/actus`
- Compiler invocation: `cargo run --quiet --bin actus -- check <file> --strict`
- Target: the current hosted/default compiler target
- Revision: `f8ad56e` (`test(phase23): close systems readiness gate`), with the
  baseline fixtures and all later Phase 23 acceptance evidence committed

## Fixtures

The minimal fixtures are stored under:

```text
/home/magradze/Projects/actus_project/actus/tests/fixtures/phase23/
```

They are intentionally not application code and do not claim that a complete
advanced systems workload is supported.

## Reproduction results

### Boolean literal expression (historical baseline)

Command:

```sh
cargo run --quiet --bin actus -- check \
  tests/fixtures/phase23/boolean_literal_baseline.act --strict
```

Observed result:

```text
error[E0003] at 2:24: expected expression, found False
```

Status at Gate 23.0: **confirmed compiler gap**. Gate 23.2 implements and
tests this capability; the command is retained here as historical evidence of
the pre-implementation failure.

The lexer recognizes `false`, but the expression parser does not yet produce a
Boolean literal expression node for this initializer path.

### Const generic array capacity (historical baseline)

Command:

```sh
cargo run --quiet --bin actus -- check \
  tests/fixtures/phase23/const_generic_baseline.act --strict
```

Observed result:

```text
error[E0003] at 6:28: expected a non-negative integer array capacity, found Identifier("N")
```

Status at Gate 23.0: **confirmed compiler gap**. Gate 23.1 implements and
tests this capability; the command is retained here as historical evidence of
the pre-implementation failure.

The parser accepts the generic declaration shape far enough to reach the array
capacity, but the type/array representation still requires a literal capacity
instead of a const-generic parameter.

### Nested indexed aggregate assignment and call

Command:

```sh
cargo run --quiet --bin actus -- check \
  tests/fixtures/phase23/nested_place_call_baseline.act --strict
```

Observed result:

```text
checked `tests/fixtures/phase23/nested_place_call_baseline.act` successfully (strict)
```

Status at Gate 23.0: **not reproduced by the minimal current fixture**.

The current compiler already accepts a nested `if` containing an indexed
aggregate field assignment and a nested named call in this reduced form. The
exact external workload that reportedly fails must be added as a minimized
fixture before parser changes are justified. Possible differences include a
const-generic receiver, a pack field, a more complex place chain, an `ins`
loan, or a branch expression rather than a statement body.

No nested-place parser gap was marked confirmed from the original report alone.
The later Gate 23.3, 23.4, and 23.5 acceptance suites now cover the expanded
place, call, and typed-join workload, including nested indexed aggregate
assignment, nested calls, and value-producing branches. This closes the
reproduction task without a speculative parser rewrite.

## Initial ownership of the work

| Capability | First inspection boundary | Evidence already present |
|---|---|---|
| Boolean literals | lexer token, expression parser, AST `Expr`, semantic type resolution, native literal lowering | negative fixture and exact `E0003` |
| Const generics | generic AST, type parser, array capacity resolver, generic identity/layout | negative fixture and exact `E0003` |
| Nested place/call path | statement parser, place AST, semantic lvalue validation, native address lowering | minimal positive fixture currently passes |

## Gate 23.0 status

- [x] Minimal fixtures exist for all three reported areas.
- [x] The two confirmed failures have exact current diagnostics recorded.
- [x] The minimal nested place/call form was tested instead of being assumed
      broken.
- [x] Minimize and reproduce the exact nested-control-flow failure, if it
      still exists in the real workload. The minimal fixture passed, and the
      later nested-place, nested-call, and typed-join suites cover the expanded
      workload without reproducing a remaining parser defect.
- [x] Add the final evidence index after the confirmed failure set is frozen.
- [x] Confirm the implementation plan and module ownership before Gate 23.1;
      the ownership table above is reflected by the implementation and test
      boundaries recorded in the later gate evidence.

Gate 23.0 is closed. The confirmed baseline gaps were implemented through
Gates 23.1 and 23.2, while the expanded nested-control-flow workload was
closed through Gates 23.3–23.5. Cross-layer and systems-readiness evidence is
indexed in [Phase 23 capability evidence index](phase-23-capability-evidence-index.md).
