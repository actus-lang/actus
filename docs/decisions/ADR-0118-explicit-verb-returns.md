# ADR-0118: Explicit Verb Returns

## Status

Accepted.

## Context

Actus makes the important facts of a program visible in the source: every
parameter carries an ownership role, every fallible call is marked, and every
resource has a visible owner. Some languages let the last expression of a
function body become its value. Actus does not: a verb returns a value only
with `return`.

Writing a value at the end of a verb body without `return` used to give the
parse error ``expected `;`, found RightBrace``, which says what the parser
saw but not what the author should write.

## Decision

A verb produces its value only with an explicit `return` statement.

A final expression without `;` yields a value only inside an expression
block, that is the branches of a value-producing `if`:

```actus
erg x = if c { 3 } else { 1 };
return x;
```

A verb body that ends in an expression without `;` is rejected with a parse
error (E0003) located at the closing `}`:

```text
expected `;` after this expression; a verb returns a value with `return`, as in `return <expression>;`
```

The message shows the fix; a conditional value is returned with
`return if v > 10 { 3 } else { 1 };`. The diagnostic applies only in a verb
that declares a return value other than `Void`. Every other missing `;`
keeps the general ``expected `;` `` error.

## Consequences

- There is one visible way for a value to leave a verb, matching the
  explicit style of the language, where ownership roles are explicit too.
- Cleanup of owned values is scheduled at `return`, which stays the single
  exit path of a verb.
- Expression blocks are unchanged.
- A verb body that ends in a bare expression was already rejected; the
  error now names the fix instead of the token the parser found.
