# ADR-0084: Labelled Unified Console Output

- Status: Accepted
- Date: 2026-10-21
- Scope: `print` console intrinsic and `std::io` migration

## Decision

Console output uses one `print` entry point with a type-directed label:

```actus
print(string: abs message);
print(int: erg value);
print(buffer: abs bytes);
eprint(string: abs message);
eprint(int: erg value);
eprint(bool: erg flag);
print(format: "score={}", int: erg value);
```

The compiler validates the label and the value type together. `string` accepts
the existing text-compatible output value, `int` accepts integer values, and
`buffer` accepts the existing borrowed byte buffer, `byte` accepts `u8`, and
`bool` accepts `Bool`.
The existing positional form, `print(value)`, remains accepted during the
migration so existing programs do not break unexpectedly.

The legacy `print_int` wrapper remains available as a compatibility alias
until all repository call sites and documentation use the labelled form.
Formatted output is deliberately bounded to one `{}` placeholder and one
`int` or `bool` value. It does not introduce variadic arguments or dynamic
allocation into engine code. `eprint` uses the same labels and writes to
stderr. This change does not alter the integer-only guarantee.

## Constraints

- No duplicate verb declarations are introduced.
- Runtime symbol selection remains type-directed and backend-neutral.
- The compiler must reject mismatched labels, such as `print(int: text)`.
- The compiler must reject unlabelled legacy `text:` output labels.
- Formatter calls must contain `format:` followed by exactly one `int:` or
  `bool:` value.
- Cranelift and LLVM must produce the same observable output.

## Verification

Semantic tests cover labelled integer and text calls, legacy positional calls,
and label/type mismatches. Native tests cover stdout output and both supported
backends before migration is accepted.
