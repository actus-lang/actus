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

- [x] Document the current `StringLiteral` AST, native data declaration, and
      `lower_string()` symbol contract.
- [x] Identify every AST visitor used by native string data collection.
- [x] Define literal deduplication and deterministic symbol naming rules.
- [x] Define the exact stdout contract for the updated AIE demonstration.

Gate 27.0 audit result: `Expr::StringLiteral` is lowered as a pointer to a
module-local `DataDescription` containing the UTF-8 bytes plus a trailing null
byte. `define_string_data()` collects literal values from reachable verbs,
deduplicates them by exact string value in a `HashSet`, sorts them before
assigning `string_0`, `string_1`, and subsequent deterministic symbols, and
`lower_string()` resolves the same value through `StringDataValues`. The
collection traversal currently covers ordinary expressions, calls, fields,
indexes, statements, loops, and direct conditional blocks, but misses case
guards, case block bodies, and `else if` branches. Gate 27.1 addresses those
specific gaps without changing the ABI or data representation.

## Gate 27.1: Complete recursive string collection

- [x] Update `src/codegen/literals.rs` so `collect_case()` visits the subject,
      every guard, expression body, and block body.
- [x] Ensure `collect_block()` recursively visits all statements that can
      contain expressions.
- [x] Ensure `collect_expression()` recursively visits `IfBranch::Block` and
      `IfBranch::ElseIf`.
- [x] Cover nested `if`/`case`, loops, returns, assignments, calls, indexing,
      casts, and aggregate literals through the shared traversal.
- [x] Keep collection independent from semantic validation and source-limit
      policy.

Gate 27.1 evidence: `src/codegen/literals.rs` now uses one shared recursive
traversal for all string-bearing control-flow paths. Statement and expression
conditionals visit both block branches and nested `else if` expressions. Case
collection visits the subject, optional guards, expression bodies, and block
bodies. The traversal continues through calls, method calls, struct fields,
indexes, casts, assignments, returns, loops, and nested blocks without
consulting semantic validation or source-limit policy. The focused regression
test `collects_strings_from_nested_control_flow` proves collection of strings
from a direct branch, an `else if`, a case guard, and a case block.
The native regression `native_string_collection_covers_case_blocks_and_else_if`
also builds and executes the same control-flow shape through `std::io`,
verifying exit code `0`, exact stdout, and empty stderr. Object-level symbol
deduplication and repeated-build determinism remain covered by Gate 27.3.

## Gate 27.2: Native data declaration and symbol integrity

- [x] Preserve the existing `DataDescription` storage and `lower_string()`
      lookup path.
- [x] Deduplicate identical literals in one native module.
- [x] Generate deterministic symbols for distinct literals.
- [x] Reject a missing collected symbol with a stable diagnostic and source
      context.
- [x] Verify object emission contains every referenced string data symbol and
      no unreachable duplicate data entries.

Gate 27.2 evidence: the existing native ABI remains unchanged: string bytes
are defined through `DataDescription`, referenced through the matching
`StringDataValues`, and lowered by `lower_string()`. Collection deduplicates
exact values before sorting them, so repeated literals share one data entry and
distinct values receive stable `string_0`, `string_1`, and subsequent symbols.
The missing-entry path remains a hard `NativeEmitError` containing the literal
value (`string literal \`...\` has no native data`), preserving actionable
literal context instead of emitting an invalid pointer. The regression
`native_string_data_is_deduplicated_and_deterministic` builds the same project
twice, compares both object files and symbol manifests byte-for-byte, and
verifies exactly two compiler-owned string data symbols for two distinct
literals despite the runtime bridge being referenced as well.

## Gate 27.3: Compiler regression and acceptance evidence

- [x] Add a native test for a String literal inside a `case` block.
- [x] Add a parser/codegen traversal regression for a String literal inside a
      case guard. Case guards intentionally reject arbitrary call expressions
      during semantic analysis, so this path is verified at the collector
      boundary rather than by forcing an invalid native program.
- [x] Add a native test for nested `if`/`case` blocks containing strings.
- [x] Verify object and executable builds.
- [x] Execute the binary and assert exact stdout.
- [x] Verify repeated literals are deduplicated.
- [x] Verify different literals receive deterministic symbols across repeated
      builds.

Gate 27.3 evidence: the native `std_io` regression builds both executable and
object artifacts for a program containing a case block, nested `if`/`else if`
branches, reachable and unreachable strings, and exact stdout assertions. The
collector unit regression covers case guards directly because the semantic
language contract rejects call expressions in guards (`InvalidGuardAccess`);
the acceptance suite does not bypass that rule with an invalid program. The
object regression builds twice, compares object bytes and symbol manifests,
and confirms deduplication and deterministic distinct data symbols.

## Gate 27.4: Readable native String example

- [x] Replace the domain-specific integer-code demo with a general Actus
      example that emits readable output:

  ```text
  ACTUS_EVENT branch=case status=ready
  ACTUS_EVENT branch=else-if status=complete
  ```

- [x] Preserve deterministic exit code `0` for the successful example.
- [x] Keep the example independent from AIE, domain-specific compiler logic,
      and private runtime behavior.
- [x] Add exact-output acceptance evidence for the example.

Gate 27.4 evidence: `examples/native_strings/` is a general hosted Actus
package using the public `std::io` facade. Its executable exercises a selected
`case` branch, an unreachable `if` branch, and a selected `else if` branch,
then emits the documented two-line stdout contract and exits with code `0`.
`tests/examples_cli.rs::native_strings_example_builds_and_emits_readable_control_flow_events`
performs strict executable build, execution, exact stdout comparison, and
empty stderr verification. No AIE-specific source, event code, or compiler
special case is involved.

## Gate 27.5: Tooling, documentation, and completion

- [ ] Keep formatter, strict check, test runner, object build, executable
      build, and LSP behavior consistent with the String contract.
- [ ] Update the coding-agent guide with the implemented String literal rules
      and examples.
- [ ] Record native object, executable, runtime stdout, and deterministic
      symbol evidence.
- [ ] Pass Rust formatting, check, clippy, tests, source limits, architecture,
      documentation, and diff checks.
- [ ] Close the phase only when no temporary Buffer workaround or opaque
      integer-code output remains in the production examples.

## Non-goals

This phase does not:

- change the String ABI or add a new runtime print bridge;
- add formatting/interpolation syntax;
- make strings mutable or heap-backed by default;
- introduce domain-specific compiler branches;
- address performance benchmarking, which remains a separate follow-up after
  readable functional output is established.

## Completion criteria

Phase 27 is complete when every native-lowerable String literal is collected
through the full AST, symbols are deduplicated and deterministic, object and
executable evidence passes with exact stdout, and the general Actus example
emits readable event descriptions while preserving deterministic exit status.
