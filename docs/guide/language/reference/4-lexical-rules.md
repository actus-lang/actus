# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 4. Lexical rules

### 4.1 Identifiers

Identifiers are names for verbs, types, fields, bindings, parameters,
constants, roles, enum variants, modules, and imports. Use descriptive
snake_case for verbs, fields, bindings, and modules. Use PascalCase for
structs, enums, packs, roles, and error domains. Use SCREAMING_SNAKE_CASE for
constants when the value is a package-level protocol constant.

Do not use generic names such as `thing`, `item`, or `data` when a domain name
is available. Names such as `frame_length`, `header_flags`, `source_buffer`,
and `remaining_bytes` are preferable.

### 4.2 Comments and documentation

Actus documentation strings use triple double quotes:

```act
"""Encode one bounded frame without allocating a second payload buffer."""
verb encode_frame(abs payload: Buffer) -> Result[Int, EncodeError] {
    ...
}
```

The repository's Actus documentation convention is `""" ... """`, not
Rust/C/C++ `//` comments. An agent must never replace Actus docstrings with
`//`, `///`, or block comments from another language. Preserve docstrings,
their position, and their line structure during formatting or refactoring.

Public structs, enums, packs, roles, constants, external bridges, and verbs
must document ownership, return values, errors, side effects, allocation
behavior, and ABI behavior where applicable. Documentation must describe what
the implementation does now, not what a future design may promise.

Actus uses `#` for ordinary source comments. Comment-only lines and inline
`#` comments are ignored by the source-size conformance metric. Triple-quoted
`""" ... """` blocks are documentation strings, not ordinary comments; they
remain part of the measured source and must be preserved for documentation
validation. Rust source keeps its normal `//` and `/* ... */` comment rules.

#### Structured verb contracts

When a verb needs a readable multi-line contract, its leading documentation
string may begin with the exact marker `contract:`. The compiler stores the
following named sections as documentation metadata and exposes them to the
formatter, semantic model, hover, completion, and signature-help tools:

```act
"""
contract:
purpose:
    Read one bounded frame.
inputs:
    source: an immutable input buffer.
outputs:
    Returns the decoded frame or a typed error.
ownership:
    The input view does not escape.
errors:
    Reports short or corrupt input.
"""
open verb read_frame(abs source: Buffer) -> Result[Frame, DecodeError] {
    ...
}
```

The supported section names are `purpose`, `inputs`, `outputs`, `ownership`,
`invariants`, `errors`, `side_effects`, and `abi`. This syntax documents the
existing behavior only: it does not add runtime checks, change ownership
analysis, or alter native lowering. Unknown, duplicate, empty contracts, or
otherwise malformed structured contracts produce parser diagnostic `E0014`.
Ordinary documentation
strings without `contract:` keep their existing meaning. See
`docs/decisions/ADR-0065-structured-verb-contracts.md` for the complete
contract and compatibility rules.

### 4.3 Whitespace and punctuation

Use semicolons after statements. Braces delimit blocks. Commas separate
parameters, fields, arguments, and generic arguments. Parentheses group
expressions and call arguments. Square brackets represent generic arguments,
array types, indexing, and generic declaration parameters. A colon separates
names from types and named arguments from values.

### 4.4 Literals

Actus supports:

- integer literals, including decimal and supported prefixed forms;
- typed integer literals such as `1u32`, `0u8`, and `255u16`;
- floating literals with `f32` or `f64` suffixes where the current type path
  supports them;
- `true` and `false` boolean literals;
- string literals;
- `Buffer[N]` construction expressions for bounded byte storage;
- `_` wildcard patterns.

The numeric suffix is part of one literal token. `1u32` is not parsed as the
integer `1` followed by an identifier named `u32`. A suffix directs the
literal's declared integer family and width; it does not perform a general
conversion of another value.

Unsuffixed integer literals can be directed by a known integer context when
the value fits. Do not rely on implicit signed/unsigned conversion between
variables. Explicit casts use `as`.

String literals are native data, not temporary text buffers. During native
emission the compiler collects every reachable `String` literal through the
complete AST, stores its UTF-8 bytes with a trailing null byte in a module-local
`DataDescription`, and lowers the expression to a pointer to that data. The
existing String ABI and `std::io` text bridge remain unchanged.

The collector traverses string-bearing expressions in declarations, returns,
assignments, compound assignments, calls, method calls, field access, indexes,
casts, aggregates, loops, nested blocks, `if` blocks, `else if` branches, and
all `case` forms including subjects, guards, expression bodies, and block
bodies. An agent must not add a special-case collector for a project or domain
module. A missing collected data symbol is a native emission error and must
not become an invalid pointer.

Exact duplicate literal values are emitted once per native module. Distinct
values are sorted before deterministic symbols such as `string_0` and
`string_1` are assigned. This makes repeated object builds reproducible and
prevents source traversal order from changing the data-symbol identity.

For readable output, use the public `std::io` facade:

```act
import std::io;

verb main() -> Void {
    erg message = "ACTUS_EVENT status=ready";
    println(abs message);
}
```

The executable example at
`examples/native_strings/src/main.act` demonstrates String literals in a
selected `case` branch, an unreachable `if` branch, and a selected `else if`
branch. Its acceptance test verifies strict build, executable output, empty
stderr, and deterministic exit status. Do not replace text output with a
Buffer workaround when a `String` is the intended API value. Case guards still
follow their semantic access rules; arbitrary calls in guards are rejected and
must not be forced into an invalid native example.

### 4.1 String views and owned UTF-8 storage

`String` and `Utf8Buffer` are intentionally different representations.
`String` is the existing immutable, null-terminated text view used by string
literals and `std::io` text output. Its native ABI is a pointer to compiler-
owned UTF-8 data with a trailing null byte. Do not treat it as an owned,
mutable byte buffer and do not return a pointer to a local buffer as `String`.

The hosted `std::string` facade provides allocation-free, borrowed inspection.
`string_length(abs text: String)` and
`string_byte_at(abs text: String, erg index: Int)` both return
`Result[Int, StringError]`. They validate UTF-8 before exposing bytes;
`StringError` distinguishes null input, invalid UTF-8, bounds, invalid storage,
and an unavailable provider. Access is byte-wise, so a multibyte code point
contributes multiple UTF-8 bytes. The `abs` role prevents mutation, ownership
transfer, and escaping views.

When bytes must become an owned text value, use `Utf8Buffer` instead of
changing the `String` ABI:

```act
import std::string;

verb make_text(dat storage: Buffer) -> Result[Utf8Buffer, StringError] {
    return utf8_from_buffer(storage: dat storage);
}
```

`utf8_from_buffer(dat storage: Buffer)` validates the caller-provided,
length-delimited buffer and transfers it only on success. For a `String`
source, `utf8_from_string(abs text: String, dat storage: Buffer)` copies into
the caller-provided storage and then returns the owned value. Both paths
expose `utf8_length(abs text: Utf8Buffer)` and
`utf8_byte_at(abs text: Utf8Buffer, erg index: Int)`, both with typed
`Result` returns. They are allocation-free
when the caller supplies storage, preserve embedded NUL bytes because the
representation is length-delimited, and release storage through normal Actus
cleanup. Capacity, invalid UTF-8, invalid storage, and bounds failures are
typed `StringError` results. Raw runtime bridges remain private; application
code uses the typed facade and `Result` errors.

For text processing, borrow `String` when reading static text, and use owned
`Utf8Buffer` when building or retaining dynamic UTF-8. A direct `String` to
owned-`String` conversion is not currently part of the language contract.
