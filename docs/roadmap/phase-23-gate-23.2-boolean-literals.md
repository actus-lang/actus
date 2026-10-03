# Phase 23 Gate 23.2: Boolean Literal Evidence

Gate 23.2 makes `true` and `false` ordinary Actus expressions. They now have
an explicit AST representation, resolve to `Bool`, preserve source spans,
format idempotently, participate in constants and conditions, and lower to
the native boolean integer representation without numeric coercion.

## Evidence

- Parser: `parses_boolean_literals_as_value_expressions` verifies both values
  are represented as `Expr::BoolLiteral` in declarations and returns.
- Semantic: `validates_boolean_literals_and_rejects_integer_bindings` accepts
  Boolean locals/constants and rejects `true` assigned to `Int`.
- Formatter: `formats_boolean_literals_without_rewriting_their_values`
  verifies canonical output and idempotence.
- Native integration:
  `executes_boolean_literals_and_short_circuit_logic_natively` evaluates
  Boolean literals through `&&` and `!` and returns exit code `42`.
- CLI fixture:
  `tests/fixtures/phase23/boolean_literal_baseline.act` now passes
  `actus check --strict` and native executable build.

## Supported positions

Boolean literals are accepted in ordinary initializers, return expressions,
conditions, logical operators, constants, and call/aggregate expressions
subject to normal type checking. They remain distinct from integer literals;
`Bool` is not silently coerced to an integer type.

## Gate result

- [x] Lexer tokens lower to explicit AST Boolean literal expressions.
- [x] Semantic type resolution and mismatch diagnostics are stable.
- [x] Native lowering and short-circuit evaluation are verified.
- [x] Formatter output is value-preserving and idempotent.
