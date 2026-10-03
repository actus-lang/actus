# Gate 23.8 Evidence: Advanced Systems Workload Readiness

Status: **closed**. The compiler accepts and natively executes a bounded,
target-neutral systems fixture that combines the language capabilities required
for the first systems workloads.

## Capability package

`examples/phase23_readiness/` is an ordinary Actus package. It contains:

- `src/main.act`: a fixed-size `Fabric[N: Usize]` graph-like aggregate;
  typed fixed-width node state; Boolean decay control; nested indexed field
  mutation; a pack-based snapshot header; Buffer serialization; typed
  `Result[Int, EncodeError]` errors; `?` propagation; and deterministic owner
  cleanup at scope exit.
- `tests/accepted.act`: a normal `meta test` discovered by the Actus test
  runner.
- `tests/fixtures/rejected_buffer_owner.act`: a negative ownership fixture
  that must reject dropping an `abs Buffer` with `E1008`.

No AIE, protocol, operating-system, or target-specific lowering is included.
The package uses the ordinary parser, semantic analyzer, generic layout,
ownership, native lowering, linker, formatter, and LSP paths.

## Commands and expected results

From `examples/phase23_readiness/`:

```text
actus check --strict
checked .../src/main.act successfully (strict)

actus test --strict
running 1 tests (strict) (0 filtered)
test ./tests/accepted.act::bounded_boolean_state ... ok
test result: ok 1 passed; 0 failed

actus fmt --check
src/main.act and tests/accepted.act are formatted

actus build --strict --emit exe -o <temporary executable>
<temporary executable>
exit code: 41

actus check tests/fixtures/rejected_buffer_owner.act --strict
error E1008: cannot drop borrow `output` directly
```

The integration test
`phase23_readiness_package_passes_strict_and_native_acceptance` executes
these commands and asserts the results. The LSP test
`lsp_understands_the_phase23_readiness_source_without_overlay_drift` opens the
same `src/main.act` and verifies empty diagnostics, formatting response, and
semantic-token response.

## Evidence map

- `executes_const_generic_array_layouts_natively` proves concrete generic
  layout specialization.
- `executes_nested_array_struct_place_assignment` proves nested indexed field
  mutation and one-address place lowering.
- `executes_boolean_literals_and_short_circuit_logic_natively` proves Boolean
  state and branch behavior.
- `executes_calls_across_nested_statement_blocks` proves nested call lowering.
- `propagates_try_from_a_nested_call_statement` proves typed error propagation.
- `phase23_readiness_package_passes_strict_and_native_acceptance` proves the
  complete bounded package path and negative ownership fixture.
- `lsp_understands_the_phase23_readiness_source_without_overlay_drift` proves
  compiler/tooling parity for the same source.

## Boundary

This gate proves compiler capability, not a production neural engine,
communication protocol, kernel, or hardware port. The fixture is intentionally
bounded and application-neutral; advanced workloads remain separate projects
that must provide their own domain-level evidence.
