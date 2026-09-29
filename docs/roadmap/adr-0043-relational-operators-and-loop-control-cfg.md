# ADR-0043 Roadmap: Relational Operators and Loop-Control CFG

## Gate 1: Relational language surface

- [x] Add relational operators to the AST and formatter.
- [x] Parse relational operators with lower precedence than arithmetic.
- [x] Infer `Bool` as the result type.
- [x] Add parser and semantic acceptance/rejection tests.

## Gate 2: Typed relational validation

- [x] Accept matching signed integer operands.
- [x] Accept matching unsigned integer operands and `Usize`.
- [x] Accept matching `f32` or `f64` operands.
- [x] Reject mixed signedness, widths, float/integer pairs, and non-numeric operands.
- [x] Add deterministic diagnostics and negative tests.

## Gate 3: Native comparison lowering

- [x] Lower signed integer comparisons with signed Cranelift conditions.
- [x] Lower unsigned integer comparisons with unsigned Cranelift conditions.
- [x] Lower floating-point comparisons with `fcmp`.
- [ ] Add native execution tests for all operator families.

## Gate 4: Nested loop-control CFG

- [x] Propagate the nearest loop target through nested blocks and `case` branches.
- [x] Preserve loop-carried values and cleanup plans for branch exits.
- [x] Add nested `break` and `continue` semantic and native regression tests.

## Gate 5: Length-aware Buffer console ABI

- [x] Route Buffer print calls to counted Buffer runtime symbols.
- [x] Remove null-terminator assumptions from Buffer output.
- [x] Verify embedded zero bytes and exact output length.
- [x] Preserve separate null-terminated String ABI behavior.

## Gate 6: Quality and documentation closure

- [ ] Update public language and runtime documentation.
- [ ] Run formatting, compilation, clippy, native tests, source limits, and diff checks.
- [ ] Close all ADR-0043 invariants with recorded command output.
