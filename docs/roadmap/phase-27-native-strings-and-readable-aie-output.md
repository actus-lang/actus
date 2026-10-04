# Phase 27: Native Strings and Readable AIE Output

Phase 27 makes native Actus string literals reliable in every expression and
control-flow position, then uses that capability to replace opaque integer
event codes in the AIE demonstration with deterministic, human-readable
output.

The phase is intentionally narrow. It does not redesign the String ABI, add a
new runtime bridge, or introduce AIE-specific compiler behavior. The existing
`DataDescription` string storage and `lower_string()` symbol lookup remain the
canonical implementation.

## Problem statement

String lowering already exists, but native data collection does not traverse
every AST branch. A string literal inside a `case` block, case guard, or nested
conditional can therefore reach lowering without a declared data symbol and
fail with:

```text
string literal `...` has no native data
```

The compiler must collect exactly the strings that can be lowered, regardless
of whether they occur in a direct expression, a guard, a block, or nested
control flow.

## Architectural contract

- String data collection and string lowering use the same literal identity.
- Collection recursively traverses the complete AST; it does not special-case
  AIE modules or source file names.
- `case` subjects, guards, expression bodies, and block bodies are collected.
- `if` block and `else if` branches are collected recursively.
- Repeated identical literals produce one deterministic data symbol.
- Different literals receive deterministic, stable symbols independent of
  traversal hash-map order.
- The existing String representation and ABI remain unchanged.
- Missing native string data is a compiler error with the originating literal
  context, never a silent empty or invalid pointer.
- Readable AIE demo output is an application-level consumer of this compiler
  capability, not a compiler special case.

## Gate 27.0: String lowering contract audit

- [ ] Document the current `StringLiteral` AST, native data declaration, and
      `lower_string()` symbol contract.
- [ ] Identify every AST visitor used by native string data collection.
- [ ] Define literal deduplication and deterministic symbol naming rules.
- [ ] Define the exact stdout contract for the updated AIE demonstration.

## Gate 27.1: Complete recursive string collection

- [ ] Update `src/codegen/literals.rs` so `collect_case()` visits the subject,
      every guard, expression body, and block body.
- [ ] Ensure `collect_block()` recursively visits all statements that can
      contain expressions.
- [ ] Ensure `collect_expression()` recursively visits `IfBranch::Block` and
      `IfBranch::ElseIf`.
- [ ] Cover nested `if`/`case`, loops, returns, assignments, calls, indexing,
      casts, and aggregate literals through the shared traversal.
- [ ] Keep collection independent from semantic validation and source-limit
      policy.

## Gate 27.2: Native data declaration and symbol integrity

- [ ] Preserve the existing `DataDescription` storage and `lower_string()`
      lookup path.
- [ ] Deduplicate identical literals in one native module.
- [ ] Generate deterministic symbols for distinct literals.
- [ ] Reject a missing collected symbol with a stable diagnostic and source
      context.
- [ ] Verify object emission contains every referenced string data symbol and
      no unreachable duplicate data entries.

## Gate 27.3: Compiler regression and acceptance evidence

- [ ] Add a native test for a String literal inside a `case` block.
- [ ] Add a native test for a String literal inside a case guard.
- [ ] Add a native test for nested `if`/`case` blocks containing strings.
- [ ] Verify object and executable builds.
- [ ] Execute the binary and assert exact stdout.
- [ ] Verify repeated literals are deduplicated.
- [ ] Verify different literals receive deterministic symbols across repeated
      builds.

## Gate 27.4: Readable AIE demonstration

- [ ] Replace opaque integer event codes in the AIE demo with readable output:

  ```text
  AIE_DEMO firing payload=41 destination=2 inhibition=1
  AIE_DEMO neurogenesis linked=1
  AIE_DEMO decay reclaimed=1
  ```

- [ ] Preserve deterministic exit code `0` for the successful demonstration.
- [ ] Keep firing, payload, inhibition, neurogenesis, linking, decay, and
      free-list behavior unchanged.
- [ ] Add exact-output acceptance evidence for the demo.

## Gate 27.5: Tooling, documentation, and completion

- [ ] Keep formatter, strict check, test runner, object build, executable
      build, and LSP behavior consistent with the String contract.
- [ ] Update the coding-agent guide with the implemented String literal rules
      and examples.
- [ ] Record native object, executable, runtime stdout, and deterministic
      symbol evidence.
- [ ] Pass Rust formatting, check, clippy, tests, source limits, architecture,
      documentation, and diff checks.
- [ ] Close the phase only when no temporary Buffer workaround or integer-code
      output remains in the production AIE demo.

## Non-goals

This phase does not:

- change the String ABI or add a new runtime print bridge;
- add formatting/interpolation syntax;
- make strings mutable or heap-backed by default;
- introduce AIE-specific compiler branches;
- address performance benchmarking, which remains a separate follow-up after
  readable functional output is established.

## Completion criteria

Phase 27 is complete when every native-lowerable String literal is collected
through the full AST, symbols are deduplicated and deterministic, object and
executable evidence passes with exact stdout, and the AIE demonstration emits
readable event descriptions while preserving its existing behavior and exit
status.
