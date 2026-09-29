# Actus Lexical Map

Version: 0.1

Status: Accepted language map

This document separates the Actus compiler's implemented vocabulary from
accepted future design and planned diagnostics. A word is not an active
keyword merely because it appears in an ADR or roadmap.

## 1. Implemented Keywords

The current lexer recognizes these keywords and the parser or semantic layer
assigns them meaning:

| Area | Keywords |
| --- | --- |
| Declarations | `verb`, `struct`, `enum`, `role`, `perform` |
| FFI and safety boundary | `extern`, `unsafe` |
| Ownership roles | `erg`, `abs`, `dat`, `ins` |
| Resource operations | `ref`, `drop` |
| Control flow | `return`, `loop`, `break`, `continue`, `case` |
| Pattern guards | `if` (only after a case pattern) |
| Modules and visibility | `import`, `open` |
| Dispatch and metadata | `dynamic`, `meta` |
| Boolean and wildcard literals | `true`, `false`, `_` |

`case` is the Actus pattern-matching construct. A general conditional statement
is not part of the language; `if` is valid only as a boolean pattern guard in
the form `Pattern if condition => body`. The current lexer also recognizes the
implemented punctuation and operators, including `->`, `=>`, comparisons,
equality, `!`, arithmetic operators, and field access with `.`.

Relational operators are `<`, `<=`, `>`, and `>=`. They bind below arithmetic
operators and produce `Bool`. The operands must belong to the same numeric
family: signed integers, unsigned integers (including `Usize`), or the same
floating-point width. Signed and unsigned values, or integers and floats, are
not implicitly converted for comparison.

`print(abs text: Buffer)` is length-aware: it writes exactly the Buffer's live
length, including embedded zero bytes, and does not require a trailing null
terminator. String output retains its separate text ABI.

`for` is reserved by the lexer but has no parser construct. `else`, `while`,
and `in` are not active language constructs. `self` remains an identifier; a
receiver role is expressed by an ordinary parameter in the current syntax.

## 2. Reserved and Future Vocabulary

These names are reserved by design documents or future work, but are not
implemented language constructs:

| Name | Planned meaning | Design location |
| --- | --- | --- |
| `act` | Owned structured task creation | ADR-0005 |
| `join` | Consume and await an owned task | ADR-0005 |
| `detach` | Explicitly transfer task responsibility | ADR-0005 |

`actor` is not a language keyword. The future `std::actor` design is an
official library specification and must use ordinary Actus declarations.

General conditional and iteration statements remain outside the current
language surface. Pattern guards are the only conditional form currently
implemented.

## 3. Reserved Future Keywords

The following names are reserved for future language features and should not
be used in new language design:

```text
async  await  yield  spawn
const  static  pragma
Self
```

`unsafe`, `extern`, `role`, `perform`, `dynamic`, `import`, `open`, and `meta`
are active and therefore do not belong in this reserved list.

The current compiler has not yet rejected every reserved name as an
identifier. Adding that enforcement is a separate lexer/diagnostics change.

## 4. Planned Banned-Word Diagnostics

These names are rejected to catch terminology imported from other languages.
The diagnostic should explain the Actus spelling:

| Banned spelling | Diagnostic hint |
| --- | --- |
| `pub` | Use `open` for visibility. |
| `use` | Use `import` for modules. |
| `fn`, `function`, `def` | Declare functions with `verb`. |
| `trait`, `interface` | Declare contracts with `role`. |
| `impl` | Use `perform` for a role realization. |
| `let`, `var` | Use `erg`, `abs`, or `dat` bindings. |
| `mut` | Mutability is expressed by the `erg` role. |
| `null`, `nil` | Actus is null-free; use `Option`. |

These hints are planned diagnostics, not current lexer behavior. Until the
diagnostic rule is implemented, the words must not be described as already
enforced bans.

## 5. Implemented Built-in Types

The current semantic built-in type registry contains:

```text
Int  Bool  String  Buffer  Array  Map
```

These are type names, not keywords. They remain available to the type
resolver without a type declaration in the source program.

## 6. Accepted Future Built-in Types

The following types are part of the accepted language design but are not all
registered in the current compiler:

```text
I8  I16  I32  I64
U8  U16  U32  U64  Usize
F32  F64
Char  Unit
Option[T]
Result[T, E]
```

`Option[T]` and `Result[T, E]` depend on the generic type phase and exhaustive
pattern semantics. They must not be documented as fully built-in until their
registry, construction, and lowering are implemented.

## Compatibility Rule

When a keyword, reserved name, banned-word diagnostic, or built-in type is
implemented, this map, the relevant roadmap item, tests, and user-facing
language documentation must be updated in the same logical change.
