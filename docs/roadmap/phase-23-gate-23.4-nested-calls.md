# Gate 23.4 Evidence: Nested Calls and Statement Parsing

Status: **closed**. The parser, semantic analyzer, and native lowering now
share the ordinary expression-statement path for calls inside nested blocks.
All implementation, evidence, and required repository quality checks pass.

## Scope

Gate 23.4 covers calls whose results are discarded, calls used as expressions,
named arguments, explicit `erg`, `abs`, `dat`, and `ins` argument markers,
postfix `?` propagation, and deterministic diagnostics for malformed nested
call statements.

## Implementation evidence

- Statement blocks require an explicit semicolon after a discarded call or
  other expression statement.
- Expression `if` blocks retain their distinct grammar: the final value
  expression may end at `}`, while statement blocks never inherit that
  implicit terminator.
- Nested calls are parsed in `if`/`else`, lexical blocks, `loop`, and `case`
  bodies without a separate parser or code-generation shortcut.
- Semantic validation preserves argument ordering, named-argument checks,
  ownership markers, unknown-verb diagnostics, and `?` return compatibility.
- Native tests cover nested `ins` buffer mutation, calls in loop/case bodies,
  and nested `?` propagation.

## Acceptance evidence

The parser suite covers all four ownership markers and nested statement
contexts, plus missing-semicolon rejection. Semantic tests cover valid nested
ownership and try propagation, invalid ownership, unknown argument names, and
unknown verbs. Native tests prove execution for nested lexical/conditional
blocks, loop/case blocks, and nested fallible calls.

Gate 23.4 is closed with this evidence intact and the full repository quality
checks passing.
