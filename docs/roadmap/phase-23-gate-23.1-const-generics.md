# Phase 23 Gate 23.1: Const Generic Evidence

This gate implements the first production slice of const generics: a
`Usize`-domain parameter supplied by a positive integer literal and consumed
as an `Array` capacity. The value remains compile-time metadata throughout
semantic analysis and native specialization; it is not stored as a mutable
runtime binding.

## Accepted source

```act
struct Cell {
    erg charge: u8,
}

struct Fabric[N: Usize] {
    erg cells: Array[Cell, N],
}

verb main() -> Int {
    erg fabric: Fabric[2] = Fabric[2] { cells: Array[Cell, 2](), };
    fabric.cells[0].charge = 41u8;
    return fabric.cells[0].charge as Int + 1;
}
```

## Evidence

- Parser: `parses_const_generic_array_capacity_for_semantic_validation`
  accepts `N: Usize` and records a const generic parameter kind.
- Semantic: `accepts_usize_const_generic_arguments_and_rejects_invalid_values`
  accepts `Table[4]` and rejects `Table[0]` and runtime identifier arguments.
- Semantic: `rejects_non_const_generic_array_capacity` rejects a role/type
  parameter used as an array capacity.
- Generic layout unit test:
  `calculates_const_generic_array_field_layout` specializes `Array[Cell, 8]`
  and verifies the concrete field size.
- Native integration test:
  `executes_const_generic_array_layouts_natively` builds and executes the
  specialized aggregate and returns exit code `42`.
- CLI fixture:
  `tests/fixtures/phase23/const_generic_baseline.act` now passes
  `actus check --strict` and native `actus build --strict --emit exe`.

## Deliberate scope boundary

The gate does not claim a general constant-expression evaluator, const
parameters in pack/layout declarations, or const-derived arena capacities.
Those positions require a separate constant-expression and backend identity
contract before they are accepted into the language.

## Gate result

- [x] AST distinguishes type and const generic parameters.
- [x] `N: Usize` parses and resolves as a const parameter.
- [x] Positive literal arguments are validated and preserved in specialization
      and layout identity.
- [x] Invalid and non-const capacities fail before code generation.
- [x] Native aggregate layout and execution are covered by regression tests.
