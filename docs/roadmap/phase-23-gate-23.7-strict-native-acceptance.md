# Gate 23.7 Evidence: Strict and Native Acceptance

Status: **closed**. The Phase 23 capability package passes strict checking,
strict test discovery, formatting validation, native executable execution, and
object inspection on the host target without a feature-specific backend path.

## Capability package

The accepted fixture is rooted at `examples/phase23_capability/`:

- `Actus.toml` declares the ordinary Alpha package and `main` entry.
- `src/main.act` combines `Fabric[N: Usize]`, Boolean branches, nested indexed
  field mutation, an in-branch call, fixed-width literals, and a typed return.
- `tests/accepted.act` is discovered by the normal `actus test` runner.
- `tests/fixtures/rejected_runtime_const.act` is intentionally invalid and
  verifies that a runtime binding cannot be used as an array capacity.

## Acceptance commands

The integration test
`phase23_capability_package_passes_strict_test_native_and_object_acceptance`
runs these commands from the package root:

```text
actus check --strict
actus test --strict
actus fmt --check
actus build --strict --emit exe -o <temporary executable>
actus build --strict --emit obj -o <temporary object>
actus check <tests/fixtures/rejected_runtime_const.act> --strict
```

Expected results are, respectively: success, one passing test, success,
success, success with a parseable object containing `.text` and `main`, and a
non-zero result for the rejected fixture. The executable returns exit code
`42`.

## Target and compiler revision

- Target profile: the host target selected by Actus (`TargetSpec::host()`),
  with the repository's default hosted native ABI.
- Compiler revision before this gate: `f6111e6`; the acceptance test executes
  the gate changes from the same working tree and is recorded by the gate
  commit that follows this evidence.
- No C bridge, handwritten lowering, target-specific dialect, or hidden
  allocation is introduced by the fixture. It uses the ordinary lexer,
  parser, semantic, native code-generation, linker, and CLI test paths.

## Evidence

- `executes_const_generic_array_layouts_natively` covers const-generic layout.
- `executes_boolean_literals_and_short_circuit_logic_natively` covers Boolean
  values and branch evaluation.
- `executes_nested_array_struct_place_assignment` covers nested indexed field
  writes and single-address place lowering.
- `executes_calls_across_nested_statement_blocks` covers nested call lowering.
- `strict_check_accepts_a_semantically_valid_source` and the package test
  above cover strict acceptance and rejection behavior.
- The full repository quality suite, source-limit checks, documentation
  checks, and diff validation are required before the gate commit.
