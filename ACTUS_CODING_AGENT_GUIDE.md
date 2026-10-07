# Actus Coding-Agent Guide

Version: 1.0

This is the operational reference for a coding agent that must read, write,
review, test, or explain Actus source code. It is intentionally more explicit
than a beginner tutorial. An agent should use it as a language contract, an
ownership checklist, a project-workflow guide, and a boundary against
inventing syntax that is only described but not implemented.

The repository is an alpha-stage, low-level systems language project. Actus is
designed for deterministic systems software, embedded and freestanding work,
binary protocols, memory-mapped data, runtime components, and bounded native
applications. The language is not a general-purpose desktop framework and the
compiler must not silently promise capabilities that are not implemented.

## 1. The most important rule for an agent

Before changing Actus code, determine which of the following is true:

1. The syntax or behavior is implemented and tested.
2. The behavior is implemented but has a known target, runtime, ABI, or
   standard-library boundary.
3. The behavior is described as a design but is not implemented.
4. The request requires a new language or compiler capability.

Only the first two categories may be used as existing Actus behavior. Category
three is a design reference, not an API. Category four requires a design and
tests before implementation. Never copy a future keyword, a future target
profile, or a future library contract into an example and present it as
working code.

When uncertain, inspect the lexer, AST, parser, semantic analyzer, native
lowering, tests, and examples in that order. A passing parser test alone does
not prove semantic validity or native execution.

## 2. Language mental model

Actus source is compiled through a one-way pipeline:

```text
source -> lexer -> parser -> AST -> semantic analysis -> ownership/cleanup
       -> native lowering -> object/linker -> executable or object artifact
```

The compiler owns four distinct concerns:

- syntax: tokens, declarations, expressions, statements, and spans;
- semantics: types, ownership roles, borrowing, module visibility, constant
  evaluation, and diagnostics;
- cleanup: deterministic destruction and scope unwinding;
- native behavior: ABI layout, bounds checks, calls, branches, and object code.

Do not move semantic rules into the lexer, ownership rules into the parser,
or backend-specific types into the AST. Do not make code generation repair an
invalid semantic program. Do not make semantic analysis print directly to the
terminal; return structured diagnostics instead.

The central language idea is that ownership is visible in source syntax. The
words `erg`, `abs`, `dat`, and `ins` are not comments and are not stylistic
annotations. They are part of the compiler-checked contract.

## 3. Minimal valid program

Hosted Alpha applications use a `main` entry verb. The current hosted entry
contract is an integer-returning main for application execution, although a
`Void` verb is also a valid language declaration and is useful for helpers and
runtime-profile examples.

```act
verb main() -> Int {
    return 0;
}
```

A source file may contain multiple verbs. The configured package entry is
selected by `Actus.toml`; hosted executable workflows currently require the
configured entry name to be `main`.

A `Void` verb has no return expression:

```act
verb initialize() -> Void {
    return;
}
```

For a statement-only helper, omitting the return type is also used by existing
source and is represented as the language's no-value return form. When adding
new code, prefer an explicit `-> Void` when the public contract should be
obvious.

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

## 5. Keywords and words

The implemented vocabulary includes the following groups.

### Declarations

- `verb` declares a callable Actus function.
- `struct` declares an aggregate with named fields.
- `pack` declares a bit/byte layout with explicit storage and field offsets.
- `enum` declares a tagged sum type with variants.
- `role` declares a callable contract/interface.
- `perform` defines role methods for a concrete target type.
- `const` declares a compile-time constant.
- `import` requests a package/module facade.
- `open` exposes a declaration through a module facade.

### Ownership and resource words

- `erg` is an owned, active binding.
- `abs` is a read-only, non-owning view/borrow.
- `dat` is a terminal ownership transfer.
- `ins` is an exclusive call-scope loan.
- `ref` creates a borrow expression.
- `drop` explicitly ends ownership and cleanup for a binding.

### Control flow and expressions

- `return` exits a verb and unwinds owned resources in affected scopes.
- `loop` creates a loop body.
- `for` creates a bounded range or fixed-array index iteration.
- `repeat` is a bounded readability alias that expands to the same `for`
  contract.
- `in` separates a `for` binding from its bounded source.
- `break` exits the nearest loop.
- `continue` starts the next iteration of the nearest loop.
- `case` performs pattern matching.
- `if` is used for an `if` statement, an `if/else` expression, and case
  guards in the implemented parser.
- `else` selects the alternate branch of an if expression or statement.
- `as` performs an explicit checked primitive integer cast.
- `true`, `false`, and `_` are boolean and wildcard syntax.

### FFI and dispatch

- `unsafe` marks a foreign or otherwise unsafe boundary.
- `extern` declares a foreign ABI boundary.
- `dynamic` requests runtime role dispatch where the callable contract allows
  it.
- `meta` attaches compile-time metadata such as `test`, `target`, or
  `limitless`.

`while`, `async`, `await`, `yield`, `spawn`, and actor-related words are not
general implemented control-flow constructs. They must not be used in new
examples unless the relevant implementation has landed. Bounded `for` and its
`in` separator are implemented only in the restricted form documented in
Section 11.3. Some other words remain reserved or planned vocabulary only.

## 6. Types

### 6.1 Built-in and primitive types

The compiler knows these central types:

| Type | Meaning | Typical use |
|---|---|---|
| `Int` | implementation-level integer used by the current hosted/runtime ABI | exit codes, runtime status values, simple counters |
| `Bool` | boolean truth value | conditions, flags, comparisons |
| `String` | text value used by the string ABI | text output and text APIs |
| `Buffer` | owned, bounded byte storage with live length and capacity | packets, files, byte-oriented I/O |
| `Array[T, N]` | fixed-capacity contiguous storage of `N` values of `T` | embedded tables, bounded state, fixed frames |
| `Map` | registered built-in type name; current native support is limited and must be checked before use | do not assume a full map API |
| `Void` | no-value return type | procedures and side-effect-only verbs |
| `Usize` | target-sized unsigned index family | array and buffer indexing |
| `u1` through `u128` | unsigned fixed-width integer families | bit fields, protocol fields, registers |
| `i1` through `i128` | signed fixed-width integer families | signed bounded arithmetic |
| `f32` | 32-bit floating-point family | only where backend and contract support it |
| `f64` | 64-bit floating-point family | only where backend and contract support it |

The parser accepts integer widths from 1 through 128 in the primitive type
registry. Native support and useful arithmetic semantics still depend on the
backend and the individual operation. Do not claim that every width has the
same ABI or instruction quality on every target.

### 6.2 User-defined types

- `struct Name { ... }` models a product/aggregate type.
- `enum Name { ... }` models a tagged union/sum type.
- `pack Name { ... }` models an explicitly laid out storage representation.
- `role Name { ... }` models a callable contract.
- `Array[T, N]` and generic declarations compose existing types.

### 6.3 Generic types

Generic parameters are written in square brackets:

```act
struct Box[T] {
    erg item: T,
}

verb identity[T](erg item: T) -> T {
    return item;
}
```

Role-bounded generic declarations use a bound:

```act
open struct BufferedReader[Source: Reader] {
    erg source: Source,
    erg buffer: Buffer,
}
```

Generic dispatch is primarily static. A concrete instantiation must have
deterministic layout and native symbol generation. Do not assume Rust-like
trait inference, specialization, associated types, or arbitrary generic
metaprogramming.

#### 6.3.1 Const generic parameters

Actus supports a production const-generic slice. A const parameter is distinct
from a type parameter and represents a compile-time bounded numeric value, not
mutable runtime storage:

```act
struct Minicolumn {
    erg charge: u8,
}

struct CorticalFabric[N: Usize] {
    erg columns: Array[Minicolumn, N],
}

verb make_fabric() -> CorticalFabric[8] {
    return CorticalFabric[8] {
        columns: Array[Minicolumn, 8](),
    };
}
```

The currently supported and tested domain is `Usize`, with positive,
representable integer literal arguments used as `Array[T, N]` capacities. The
compiler preserves the const argument in generic identity, layout identity,
specialized native symbols, and deterministic cache/build decisions.

The following are required compiler behaviors:

- `N: Usize` is parsed as a const parameter, not as an invalid primitive type
  or an ordinary type bound;
- the argument is resolved before semantic layout and native lowering;
- runtime expressions are rejected in const-generic positions;
- negative, zero, overflowing, or otherwise unrepresentable capacities are
  rejected before code generation;
- the const value is not silently allocated as mutable runtime state;
- missing arguments, wrong domains, duplicate parameters, and unresolved
  const references receive deterministic diagnostics.

Const expressions and additional compile-time positions beyond the bounded
`Array` capacity slice are not silently implied by this feature. However, once
a const parameter is declared on a generic verb, its value may be used as a
compile-time expression value inside that verb:

```act
verb capacity[N: Usize]() -> u32 {
    if 0u32 < (N as u32) {
        return N as u32;
    }
    return 0u32;
}

verb main() -> Int {
    return capacity[4]() as Int;
}
```

The compiler specializes `N` to the concrete literal before native lowering.
This works through casts, binary expressions, conditions, indexing, and
nested expression arguments. Explicit generic verb calls use the same square
bracket syntax as generic type applications. A const argument must still be
a positive compile-time `Usize` literal or an already-declared const
parameter; runtime expressions, zero, nested type applications, and
reference-role-qualified arguments are rejected.

Do not assume that arbitrary constant arithmetic is accepted as a generic
argument, or that a const parameter is a mutable runtime binding. Use a
named `const` for reusable compile-time expressions outside a generic
parameter's specialization contract.

### 6.4 `Option` and `Result`

The compiler supplies the canonical generic enum shapes:

```act
enum Option[T] {
    Some(T),
    None,
}

enum Result[T, E] {
    Ok(T),
    Err(E),
}
```

Use `Option[T]` when a value may be absent without using null. Use
`Result[T, E]` when an operation can succeed or fail with a typed error.

```act
verb lookup(abs input: Buffer) -> Option[Int] {
    return Option[Int].Some(7);
}

verb read_value() -> Result[Int, IoError] {
    return Result[Int, IoError].Ok(7);
}
```

Variant construction uses `Type[Arguments].Variant(...)` for a known generic
type or `Result.Ok(...)`/`Result.Err(...)` where inference has enough context.
Pattern matching extracts payloads. `?` propagates a compatible `Err` from a
fallible expression.

When native lowering extracts a regular enum payload from another enum, it
materializes an owned payload allocation before the binding is used. This keeps
`dat` transfers and cleanup on valid allocation boundaries; inline struct
payloads continue to use their containing storage directly. Compiler changes
must preserve this distinction and cover both accepted execution and cleanup
regressions.

## 7. Ownership roles

### 7.1 `erg`: active owner

`erg` declares the binding that owns a resource or value and may mutate it.

```act
erg packet: Buffer = Buffer[128];
append(packet, 0x42);
```

An `erg` binding may be read, mutated, borrowed, passed by explicit role,
moved into another owner, or dropped. It is the normal local role for mutable
state.

### 7.2 `abs`: read-only view

`abs` creates or accepts a non-owning read-only view:

```act
verb checksum(abs packet: Buffer) -> Int {
    return length(packet);
}
```

The view does not consume the source, cannot mutate it, and cannot outlive its
origin. An owner with an active `abs` borrow is frozen until the borrow ends.

Explicit borrow construction uses `ref`:

```act
abs view = ref packet;
```

### 7.3 `dat`: terminal transfer

`dat` transfers ownership into the callee:

```act
verb consume(dat packet: Buffer) -> Int {
    erg size = length(packet);
    drop(packet);
    return size;
}

erg packet = Buffer[16];
consume(packet: dat packet);
```

After the transfer the caller binding is `Moved` and cannot be read, mutated,
borrowed, or dropped again. The receiving scope becomes responsible for
cleanup.

### 7.4 `ins`: exclusive call-scope loan

`ins` temporarily gives a callee exclusive mutation access while the caller
retains ownership after the call:

```act
verb append_marker(ins packet: Buffer) -> Result[Int, IoError] {
    append(packet, 0x7f);
    return Result[Int, IoError].Ok(1);
}

erg packet = Buffer[16];
append_marker(packet: ins packet)?;
append(packet, 0x01);
```

During the call the source owner is suspended. It cannot be read, moved,
borrowed, or passed to another exclusive loan. After the call it returns to an
active mutable state. `ins` is a parameter/call role, not persistent struct
storage. An `ins` field is rejected because a call-scoped loan cannot be
stored as aggregate state.

### 7.5 Ownership state transitions

An agent must reason about these states:

```text
Active erg -> Frozen while abs borrow exists -> Active after borrow ends
Active erg -> Suspended during ins call       -> Active after call returns
Active erg -> Moved after dat transfer
Active erg -> Dropped after drop or scope cleanup
```

Illegal transitions include mutation of a frozen owner, use of a suspended
owner, use after move, use after drop, aliased `ins`, double cleanup, and a
borrow escaping its origin.

### 7.6 Cleanup

Owned values are cleaned in deterministic reverse declaration order. Cleanup
also occurs before `return`, `break`, `continue`, and `?` transfer control out
of the affected scope. Moved fields and explicitly dropped fields are removed
from the cleanup plan exactly once.

Do not insert manual cleanup to work around the compiler. First model the
ownership role correctly. Use `drop` only when an early, intentional release
is part of the operation.

## 8. Verb declarations and calls

### 8.1 Ordinary verbs

```act
verb add(erg left: Int, erg right: Int) -> Int {
    return left + right;
}
```

Parameters have a role, a name, and a type. The return type follows `->`.
Named call arguments are preferred because they make ownership explicit:

```act
erg total = add(left: 2, right: 3);
```

### 8.2 Visibility

`open` makes a declaration eligible to cross a module facade:

```act
open verb public_checksum(abs packet: Buffer) -> Int {
    return checksum(packet: abs packet);
}
```

An unmarked declaration is private to its source/module boundary. `open` does
not mean “global”; the canonical facade still has to re-export a sibling
declaration.

### 8.3 Method-like calls

The compiler supports method-like calls for role/performance and aggregate
operations where the receiver contract exists. The current language also
allows ordinary verbs with a named receiver parameter. Do not invent a
separate `self` keyword; `self` is not a special receiver keyword.

### 8.4 Static and dynamic dispatch

Static dispatch is the default. A parameter may specify dynamic role dispatch
where a role contract supports it:

```act
verb write_all[Target: Writer](ins target: Target, abs bytes: Buffer) -> Result[Int, IoError] {
    return write(target: ins target, buffer: abs bytes);
}
```

The exact runtime dispatch form must follow existing role/perform and standard
library examples. Dynamic dispatch is an explicit ABI/runtime boundary, not a
reason to put backend-specific vtables in the AST.

## 9. Structs and aggregates

### 9.1 Struct declaration

```act
struct Point {
    x: Int,
    y: Int,
}
```

Fields may be value fields or ownership-role fields:

```act
struct PacketState {
    erg payload: Buffer,
    abs header: Buffer,
}
```

`erg` fields are owned mutable subresources and participate in aggregate
cleanup. `abs` fields are non-owning read-only views. `ins` fields are not
allowed. `dat` is used at operation boundaries, not as persistent field state.

### 9.2 Struct construction and access

```act
erg point = Point { x: 10, y: 20 };
erg x = point.x;
point.x = x + 1;
```

Field access uses `.`. Field assignment is ownership-checked. Partial field
moves are tracked. Whole-struct assignment transfers ownership rather than
implicitly copying a resource-bearing aggregate.

### 9.3 Struct methods through `perform`

```act
role Writer {
    write(abs bytes: Buffer) -> Result[Int, IoError];
}

perform Writer for DeviceWriter {
    open verb write(abs bytes: Buffer) -> Result[Int, IoError] {
        ...
    }
}
```

The role declaration is the contract. The `perform` block supplies methods for
the target type. The implementation must satisfy parameter roles, types,
return types, visibility, and dispatch rules.

## 10. Enums and pattern matching

### 10.1 Enum declaration

```act
enum DecodeError {
    Empty,
    InvalidLength,
    Checksum(Int),
}
```

Variants may be unit, tuple, or named-field payloads:

```act
enum Message {
    Ping,
    Data { length: u16, flags: u8 },
}
```

### 10.2 `case`

`case` is the pattern-matching construct:

```act
erg result = read_value();
case dat result {
    Result.Ok(value) => process(dat frame),
    Result.Err(error) => {
        return report_error(error: dat error);
    },
};
```

The `abs` or `dat` mode after `case` controls how the subject is inspected or
transferred where the contract permits. Use `_` for a wildcard:

```act
case status {
    0 => 0,
    _ => 1,
};
```

A boolean guard may follow a pattern:

```act
case value {
    number if number > 0 => number,
    _ => 0,
};
```

The branch body may be an expression or a block. Branches must satisfy the
expected type when used as an expression. Diverging branches such as `return`
do not produce a value and do not incorrectly force an unrelated type join.

### 10.3 `Option` and `Result` patterns

Use qualified variants when the type context is ambiguous:

```act
case dat decoded {
    Result.Ok(frame) => process(dat frame),
    Result.Err(error) => recover(dat error),
};
```

The `_` pattern deliberately ignores a value. If the ignored value owns a
resource, verify that the compiler's cleanup plan is correct; do not hide an
ownership transfer in a wildcard without testing it.

## 11. Conditions, loops, and assignment

### 11.1 If statements

```act
if index >= count {
    break;
}
```

The condition must be `Bool`. Branches are normal blocks and preserve
ownership/cleanup semantics.

### 11.2 If expressions

```act
erg selected: u32 = if ready {
    41u32
} else {
    7u32
};
```

Both value-producing branches must unify to a compatible type. A diverging
branch may be used where the other branch determines the expression type.
Nested and `else if` forms must be tested through parser, semantic, and native
execution layers.

### 11.3 Loops

```act
loop {
    if done {
        break;
    }
    continue;
}
```

`break` and `continue` target the nearest enclosing loop. They unwind local
owned values correctly. Loop-carried state must be initialized and updated in
an ownership-safe way.

Actus also supports two bounded `for` forms:

```act
for erg index: u32 in 0u32 .. 8u32 {
    total += index;
}

for erg index: u32 in values {
    total += values[index];
}
```

The range is half-open: `start` is included and `end` is excluded. Both range
bounds must have the declared primitive integer type of the `erg` index
binding. Empty and reversed ranges execute zero times. Values that overflow
the declared bound type are rejected during semantic analysis.

The collection form is index iteration over a statically known `Array[T, N]`.
It also supports pack fields backed by fixed arrays. The compiler derives the
finite bound from the array layout; the loop does not discover a runtime
length. The binding is an explicit `erg` integer index. `abs`, `ins`, and `dat`
bindings are parsed but rejected by semantic analysis in the current profile.

Dynamic collections, scalar sources, unbounded iterators, runtime length
discovery, and hidden collection allocation are not valid `for` sources. Native
lowering emits integer condition, body, increment, and exit blocks. `continue`
always targets the increment block, so it cannot skip index advancement. The
existing cleanup plan applies to `break`, `continue`, nested loops, and early
return. Formatter, definition lookup, and hover preserve and expose the loop
binding like other local bindings.

`repeat erg index: u32 in start .. end { ... }` is a readability helper for the
same bounded range contract. The compiler expands it to canonical `for` form
before semantic analysis; formatter output uses the canonical `for` spelling.
It does not add callbacks, allocation, dynamic bounds, or a second ownership
model.

### 11.4 Assignment

Simple assignment:

```act
counter = 1u32;
packet[index] = byte;
state.flags = state.flags | 0x01u8;
```

Compound assignment is supported for scalar, field, index, and pack-field
places where the operator and types are valid:

```act
index += 1u32;
state.flags |= 0x01u8;
packet[offset] ^= mask;
```

The left-hand place must be evaluated exactly once. Native lowering must
calculate an address, load, operate, and store through that same place. Never
rewrite a compound assignment into a duplicated side-effecting index or field
calculation by hand.

## 12. Operators

Precedence, from tightest to loosest:

| Level | Operators |
|---:|---|
| 1 | unary `!`, `~`, `-` |
| 2 | `*`, `/`, `%` |
| 3 | `+`, `-` |
| 4 | `<<`, `>>` |
| 5 | `&` |
| 6 | `^` |
| 7 | `|` |
| 8 | `<`, `<=`, `>`, `>=` |
| 9 | `==`, `!=` |
| 10 | `&&` |
| 11 | `||` |

Binary operators are left-associative. Parentheses override precedence.

### Numeric operators

`+`, `-`, `*`, `/`, and `%` operate on compatible numeric types. Integer
families preserve declared width and signedness. Do not expect automatic
promotion between `u8`, `u32`, `Int`, or signed/unsigned families.

### Comparisons

`<`, `<=`, `>`, and `>=` return `Bool`. Operands must belong to the same
numeric family: matching signed integers, matching unsigned integers, or the
same floating width. Signed and unsigned values are not implicitly mixed.

`==` and `!=` compare compatible numeric families or booleans.

### Logical operators

`&&` and `||` require booleans and short-circuit the right side. `!` requires
and returns `Bool`. Do not use bitwise operators as boolean operators unless
the operands are explicitly integer masks.

### Bitwise and shift operators

`&`, `|`, `^`, and `~` require integer operands and preserve the validated
width/signedness. Shifts require an integer left operand and an unsigned count.
An invalid count traps at runtime or is rejected statically when known.

### Division and remainder

Division and remainder by zero are deterministic failures. A statically known
zero divisor is rejected during semantic analysis. A dynamic zero divisor is
handled by the native runtime failure path.

## 13. Casts and numeric safety

The explicit cast form is:

```act
erg byte: u8 = value as u8;
```

`as` is a checked primitive integer cast. It does not allocate and does not
change ownership. Constant values outside the target range are rejected
during semantic analysis. Dynamic values use a native range check and fail
deterministically on overflow or underflow.

There is no general implicit numeric conversion. If a `u32` operation needs a
`u32` operand, write `1u32` or use an explicit cast. Do not solve type errors by
changing a protocol field to `Int` unless the ABI and width contract really
requires it.

## 14. Constants and compile-time data

Named constants are compile-time values:

```act
const MAX_FRAME: u16 = 1024u16;
const HEADER_SIZE: u8 = 8u8;
const DATA_MASK: u32 = 0x00ff_ffffu32;
```

Constants are intended for protocol masks, limits, widths, layout values, and
other data that should not become mutable runtime storage. The compiler
validates constant types, ranges, dependencies, cycles, and compile-time
availability. Do not use a runtime variable when a protocol invariant is
truly fixed.

Compile-time constants do not make arbitrary runtime computation compile-time.
Do not rely on a constant evaluator to read files, call foreign functions,
inspect hardware, or access mutable state.

Pack field offsets may name a checked integer constant:

```act
const PAYLOAD_OFFSET: u16 = 8u16;

pack Frame {
    erg storage: Array[u8, 2];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg payload: u8 at PAYLOAD_OFFSET;
    }
}
```

Named offsets may chain through integer constants and checked integer
arithmetic. They must resolve before native lowering and remain within the
`u16` layout-offset domain. Calls, runtime reads, allocation, mutation,
buffers, strings, booleans, floating point, and dynamic lengths are invalid in
layout constants. The numeric `at 0` form remains valid for literal offsets.

## 15. `Buffer`, `Array`, `pack`, and `Arena`

### 15.1 `Buffer`

`Buffer` is the primary owned byte container. It has a live length and a
capacity and is used for protocol frames, file data, streams, and raw I/O.

```act
erg buffer: Buffer = Buffer[128];
append(buffer, 0x42u8);
erg length = buffer_length(buffer: abs buffer)?;
```

Buffer indexing is checked against the live range. A zero-length buffer is
valid, but reading or writing an index outside the live range is invalid. Use
the standard-library buffer operations or documented runtime intrinsics for
reserve, append, clear, length, and range operations. Do not assume a null
terminator; binary output uses the live length.

`abs Buffer` reads without consuming or allocating. `ins Buffer` mutates the
caller-owned storage in place. `dat Buffer` transfers cleanup responsibility.

### 15.2 `Array[T, N]`

`Array[T, N]` is contiguous, fixed-capacity storage:

```act
erg values: Array[u32, 4] = Array[u32, 4]();
values[0] = 10u32;
values[1] += 1u32;
```

Indexing accepts `Int`, `Usize`, and unsigned integer indices where the
semantic contract permits. Capacity and runtime bounds are checked before
native load/store. Array slots can participate in exclusive `ins` loans without
copying the entire array.

### 15.3 `pack`

`pack` expresses an explicit storage layout for registers, headers, flags, and
protocol fields. Follow the accepted repository form when adding new code:

```act
pack Register {
    erg storage: u8;
    layout little;
    fields {
        erg value: u8 at 0;
    }
}
```

Pack fields are read and written through field access. The compiler performs
shift/mask lowering, validates field widths and offsets, and preserves the
declared endianness/layout contract. Do not manually duplicate bit shifts when
a pack declaration expresses the actual representation.

### 15.4 Declarative serialization contracts

Fixed-width binary formats may be declared with `serialize` when the byte
layout is part of the type contract:

```act
serialize Frame from FramePack {
    layout little;
    version u16 at 0;
    payload bytes at 2 length 16;
    checksum crc32 over 0 .. 18 at 18;
}
```

The first accepted profile requires exactly one `version u16`, one fixed-size
payload, and one `crc32` section. Offsets, lengths, checksum ranges, and the
source pack's byte capacity are validated at compile time. Sections may not
overlap, and the checksum field may not overlap its input range. The
declaration does not allocate memory, open files, or perform I/O. Generated
read, write, validation, and migration operations must use caller-owned
buffers and explicit ownership roles. The `crc32` intrinsic computes an IEEE
CRC32 over a validated buffer range, while `crc32_matches` compares that value
with an expected integer. The `validate_fixed_frame` intrinsic exposes the
fixed-frame runtime validator, which checks version,
payload bounds, endianness, and stored CRC without allocation. These operations
do not allocate. Generated serialization operations use the compiler-provided
`SerializationError` enum. The compiler currently generates
`<contract>_validate`, which accepts a caller-owned `Buffer` and a read-only
expected version, and `<contract>_encode`, which accepts an `abs` source pack
and an `ins` output `Buffer`, copies the fixed storage bytes, and returns a
typed `Result[u32, SerializationError]`. It also generates
`<contract>_decode`, which accepts an `abs expected_version: u16`, checks the
input length through the compiler-provided `buffer_length` primitive, validates
the encoded version and checksum, and returns an owned pack or a typed
`InvalidLayout`, `InvalidVersion`, or `InvalidChecksum` error. Both operations
derive their fixed storage bounds from the declaration. The compiler also
generates `<contract>_migrate`, which validates `from_version`, copies the fixed
frame into an empty caller-owned output, writes `to_version`, recomputes the
declared checksum, and returns a typed byte count. Dynamic payloads, implicit
allocation, and automatic filesystem commits remain outside this profile until
their contracts are implemented and tested.

When generated migration is available, use the explicit form
`<contract>_migrate(abs input: Buffer, abs from_version: u16,
abs to_version: u16, ins output: Buffer)`. It validates the source version,
rewrites the target version, recomputes the declared checksum, and writes into
the caller-owned output. The output must be empty before migration. Migration
does not perform filesystem I/O; atomic file replacement belongs to the
filesystem persistence layer.

### 15.5 `Arena[N]`

The native backend contains bounded arena support used by current aggregate
and systems examples:

```act
erg arena: Arena[64] = Arena[64]();
erg slot = arena.place(value: Array[Int, 2]());
```

Arena capacity and cleanup are explicit. Treat this as a bounded storage
primitive, not a garbage collector or an unbounded heap. Verify the exact
constructor and method contract against current examples before using it in a
new public API.

## 16. Foreign functions and unsafe boundaries

Foreign declarations require an explicit boundary:

```act
unsafe extern "C" verb device_read(ins buffer: Buffer) -> Int;
```

`unsafe` tells the compiler and reviewer that safety depends on an external
contract. `extern "C"` selects the C ABI. The initial supported C boundary is
restricted and layout-checked; current documented support includes `Int` and
`Buffer` pointer-like contracts, with role-specific restrictions for `abs`,
`dat`, and `ins`.

Rules for an external bridge:

1. Document the exact ABI, status values, ownership role, and side effects.
2. Keep the raw bridge private behind a typed Actus facade whenever possible.
3. Translate raw statuses into `Result[T, E]` before exposing the API.
4. Never encode errors through undocumented arithmetic such as `status - 1`.
5. Validate pointer/resource handles and layout assumptions at the boundary.
6. Add accepted and rejected native execution tests.

The standard library uses this pattern: private `unsafe extern "C"` bridges,
public typed wrappers, and facade-controlled exports.

## 17. Modules, imports, and facades

### 17.1 Importing

Use extensionless canonical module paths:

```act
import std::io;
import std::fs;
import std::path;
```

The package/build configuration resolves the module package. Do not hard-code
an absolute host path to `library/std` in application source.

### 17.2 Canonical facades

A directory module has one facade whose filename matches the directory:

```text
library/std/src/io/io.act
library/std/src/fs/fs.act
library/std/src/path/path.act
```

The facade re-exports siblings with extensionless `open` entries:

```act
open stdout;
open stdin;
open error;
```

Sibling files share internal module scope. External users see only declarations
exposed through the canonical facade. Direct sibling bypasses, missing
facades, duplicate declarations, and malformed exports must be rejected.

### 17.3 Package configuration facade

`src/config/config.act` is the reserved package configuration facade when a
package uses source-level configuration:

```act
// src/config/config.act
open values;

// src/config/values.act
open const DEFAULT_THRESHOLD: u8 = 30u8;
```

Normal package code consumes it through `import config;`. Configuration files
are compile-time-only and may contain typed constants plus supporting
type-level declarations, but they must not import application, runtime,
hardware, or standard-library modules, and must not declare verbs, external
verbs, roles, or performances. The canonical facade controls visibility;
private constants and direct child-module imports remain unavailable.

Use `Actus.toml` for package, build, target, and runtime configuration. Do not
put mutable runtime state or deployment settings in `src/config`. The compiler,
formatter, LSP, test runner, and native backend must all resolve this facade
through the same module boundary.

#### 17.3.1 Configuration constants in native modules

Configuration constants remain compile-time values after they cross the
facade boundary. Native lowering must resolve them in the same public export
namespace used by semantic analysis; replacing them with source-level
literal workarounds is incorrect.

The native object plan must collect compile-time constants from the full
transitive canonical-facade dependency graph. Only public exports may enter
the imported object program; private constants and facade-bypass declarations
remain unavailable. Constants must be materialized before native identifier
lowering and must never become runtime storage or native ABI symbols.

This contract also applies through nested module facades. A child module may
import the package configuration facade and use its public constants when the
final native object is emitted for the parent module. The compiler must carry
the constant into the corresponding module object before native semantic
analysis and lowering.

Package constants may be used in all supported compile-time expression
contexts, including:

- scalar initializers and assignments;
- struct and pack literals;
- predicates, `case` guards, and conditional expressions;
- array capacities and generic const arguments where the type contract allows
  them;
- indexed expressions and layout-related values.

A constant identifier is not a runtime binding and has no ownership state.
Using a constant as an `erg` field or aggregate initializer value must not
attempt to move, borrow, or drop a runtime owner. Constants remain typed and
range-checked during semantic analysis, then are inlined or otherwise
materialized before native lowering.

When changing package configuration or module aggregation, verify the full
boundary rather than only source checking:

```sh
actus check --strict
actus build --strict --emit obj
actus build --strict --emit exe
actus test --strict
```

At least one regression test must cover a constant imported through a nested
facade and used in an aggregate initializer or predicate. The test should
verify native execution, not only semantic acceptance.

#### 17.3.2 Native dependency closure

Native emission starts from the requested entry verb or public facade roots
and computes a deterministic transitive closure of Actus verbs. Reachable
private helpers are emitted in their owning object; unreachable private verbs
are omitted. Generic verbs are specialized before declaration and lowering,
and concrete instances retain deterministic names across root-to-module
linker bindings.

The closure includes nested blocks, conditional expressions, indexed places,
method/performance calls, aggregate return dependencies, and external bridge
declarations. Built-in constructors and runtime intrinsics are not Actus verb
bodies. An unresolved native dependency fails closed at the originating call
span and identifies both the caller and missing helper; it must not be hidden
until linking.

Object builds, executable builds, and the test runner use this contract.
For imported module objects, native roots come from the resolved facade export
set, including concrete generic instances when available; they must not be
inferred from source-limit metadata or from a declaration's local `open` flag.
Formatter and source-limit checks remain independent of native reachability.
Changes to call collection, generic specialization, module facades, or symbol
bindings require semantic, native, and multi-object regression evidence.

#### 17.3.3 Shared resolution boundaries

`actus check --strict` and the LSP use the shared module/facade semantic
resolution contract. They must resolve sibling declarations, nested facades,
visibility, and public exports consistently, but they intentionally stop before
native code generation.

`actus test --strict`, object builds, and executable builds use the shared
native dependency-closure contract described above. They must start from the
same selected public roots, materialize the same reachable private and generic
dependencies, and preserve the same symbol and diagnostic identity across
surfaces. Do not implement a separate dependency scanner for one command or
force semantic-only commands to invoke code generation.

### 17.4 Visibility

`open` on a declaration or sibling export is the Actus visibility mechanism.
Do not use Rust `pub`, Go `export`, or C header conventions in Actus source.
Fields do not have a separate `pub` keyword; aggregate visibility follows the
declared facade and type contract.

### 17.5 Hierarchical facades

When a module grows beyond a single responsibility, use nested canonical
facades instead of making implementation files independently importable:

```text
src/control/control.act
src/control/runtime/runtime.act
src/control/runtime/safety.act
```

`control.act` may contain `open runtime;`, and `runtime.act` may contain
`open safety;`. External code imports only `control`. Every child directory
must contain a matching `<directory>/<directory>.act` facade. Child siblings
share internal scope, while only declarations explicitly opened through the
facade chain are public.

Valid application code uses `import control;`. Direct child paths such as
`import control::runtime;`, implementation paths such as
`import control::runtime::safety;`, and facade filenames as import segments
are invalid because they bypass the parent-controlled API. Keep the hierarchy
target-neutral and responsibility-oriented; filesystem enumeration must never
silently widen the public namespace.

## 18. Roles and performance implementations

A role is a compile-time callable contract:

```act
open role Reader {
    read(ins buffer: Buffer) -> Result[Int, IoError];
}
```

An implementation associates the role with a concrete type:

```act
perform Reader for Cursor {
    open verb read(ins buffer: Buffer) -> Result[Int, IoError] {
        return cursor_read(self: ins self, buffer: ins buffer);
    }
}
```

Role methods must preserve ownership roles and return contracts. Static role
dispatch is the normal embedded-friendly path. Dynamic dispatch is an explicit
runtime contract and must not be introduced just to avoid a generic type error.

## 19. Metadata

Metadata is written with `meta` and is compiler-facing, not a runtime value.

### Tests

```act
meta test
verb buffer_contract() -> Int {
    ...
}
```

The Actus test runner discovers test verbs according to the current source
test contract. Keep fixtures focused and deterministic.

### Target selection

```act
meta target("host")
verb host_only_entry() -> Int {
    ...
}
```

Target metadata filters declarations for a configured compiler target. Target
profiles belong to compiler/build/runtime configuration, not to the core
language syntax. Never embed a list of CPU models into a type or operator.

### Source-limit exceptions

Source-size limits measure code-bearing lines rather than raw physical lines.
Blank lines and comments are excluded, while Actus documentation strings are
counted. The preferred, split-required, and hard thresholds therefore apply to
the actual source structure and are not increased merely by adding comments.

The recommended default is to keep source files and verbs within the project
limits. If a carefully justified exception is needed:

```act
meta limitless("verb")
verb generated_protocol_decoder() -> Result[Int, DecodeError] {
    ...
}
```

At file scope:

```act
meta limitless("file")
"""Generated compatibility surface; reviewed as one boundary."""
```

The package-wide policy is configured in `Actus.toml` as part of `[package]`:

```toml
[package]
name = "example"
version = "0.1.0"
edition = "alpha"
entry = "main"
source_limits = "limitless"
```

Do not add `source_limits = "default"` redundantly. Default limits apply when
the package does not opt out. A limitless policy is a reviewed exception and
should not become the normal design strategy.

## 20. Standard library

The standard library is selected through the package build/runtime setting.
New projects default to the dependency-free `core` runtime. A project that
needs hosted standard-library modules opts into `std`:

```toml
[build]
runtime = "std"
```

The compiler resolves canonical `std::io`, `std::fs`, `std::path`, and
`std::string` imports
from the packaged library. Applications must use public typed facade APIs,
not the internal C bridge symbols.

For a package that requires an integer-only generated native boundary, enable
the compiler-owned Cranelift IR audit in `Actus.toml`:

```toml
[build]
verify_no_float_ir = true
```

The setting applies to every Actus-generated native function emitted for the
package and rejects floating-point IR before object emission. It proves only
the generated Actus IR boundary. External ABI objects and separately linked
runtime objects need their own audit and are not silently covered by this
setting.

### 20.1 `std::io`

The public I/O facade includes:

- `IoError` typed error enum;
- `Reader` and `Writer` role contracts;
- `read(ins buffer: Buffer) -> Result[Int, IoError]`;
- `write(abs buffer: Buffer) -> Result[Int, IoError]`;
- `read_line(ins buffer: Buffer) -> Result[Int, IoError]`;
- `read_byte() -> Result[Int, IoError]`;
- `print(abs text: String) -> Result[Int, IoError]`;
- `println(abs text: String) -> Result[Int, IoError]`;
- `print_int(erg value: Int) -> Result[Int, IoError]`;
- `printb(abs text: Buffer) -> Result[Int, IoError]`;
- `printlnb(abs text: Buffer) -> Result[Int, IoError]`;
- `eprint_int`, `eprint`, and `eprintln` for stderr;
- `flush()` for stdout;
- `Cursor` and cursor read/write/seek/flush operations;
- buffered reader and writer types and operations;
- `copy` for reader-to-writer transfer.

Use `print` for `String`, `printb` for binary `Buffer`, and typed result
handling for failures. Do not pass a `String` to `printb` or a `Buffer` to
`print` without an explicit, documented conversion boundary.

`Result.Ok(count)` is the number of bytes processed, not merely a boolean
success flag. EOF is a typed result condition where the API defines it.

### 20.3 `std::string`

`std::string` is hosted-only until a freestanding target supplies the same
checked provider contract. It contains `StringError` for null, invalid UTF-8,
bounds, invalid-storage, and provider failures; borrowed byte inspection for
the existing `String` ABI; and `Utf8Buffer`, an owned length-delimited UTF-8
value backed by a caller-supplied `Buffer`.

Use `utf8_from_buffer(dat storage: Buffer)` for validation and ownership
transfer, or `utf8_from_string(abs text: String, dat storage: Buffer)` for a
caller-buffer-backed String-to-owned-UTF-8 conversion. Use
`utf8_length(abs text: Utf8Buffer)` for the exact byte length, and
`utf8_byte_at(abs text: Utf8Buffer, erg index: Int)` for checked byte access.
Use `std::io::print`/`println` for `String` and `printb`/`printlnb` for raw
`Buffer` bytes; these representations must not be confused.

### 20.2 `std::time`

Use `std::time` for elapsed-time measurement and explicit monotonic deadlines:

```act
import std::time;

verb measure() -> u64 {
    erg started: Instant = now();
    erg finished: Instant = now();
    erg elapsed = duration_since(later: abs finished, earlier: abs started);
    return case dat elapsed {
        Result.Ok(value) => duration_as_nanos(abs value),
        Result.Err(_) => 0u64,
    };
}
```

`Instant` is not a wall-clock timestamp. Use `Duration` for checked integer
units and arithmetic, `Deadline` for expiration, `delay` for explicit
busy-waiting, `sleep` for provider-backed scheduler cooperation, and `Timer`
for caller-owned one-shot or periodic state machines. Handle every
`Result[_, TimeError]`; do not use a raw runtime symbol or assume nanosecond
hardware precision. A fixed-workload benchmark must document target, runtime,
optimization, workload, and scheduling conditions. The general executable
example is `examples/monotonic_time/`.

### 20.3 `std::fs`

The filesystem facade includes:

- `File` owned file handle;
- `OpenOptions` for read, write, append, truncate, create, and create-new;
- `SeekFrom` and `Seeker`;
- `Metadata` and `MetadataProvider`;
- `file_open`, `file_create`, `file_read`, `file_write`, `file_flush`,
  `file_close`, and file seek operations;
- `read_to_bytes` and `read_to_string`;
- `write_file` and `write_file_atomic`;
- `remove_file`, `rename`, `copy_file`, `create_dir`, and `remove_dir`;
- metadata access through paths and file handles.

All public operations return typed `Result` contracts. Files and buffers are
owned resources and must be passed with the correct role. A failed operation
must preserve the documented owner and cleanup behavior.
`write_file_atomic` consumes a caller-selected staging `Path`, flushes the
staging file, renames it to the final destination, and removes staging on
failure. Manifest publication and directory durability remain caller policy.

### 20.4 `std::path`

The path facade includes:

- owned `Path` and `PathPlatform` representations;
- `PathError` typed failures;
- POSIX and Windows root classification;
- validated path construction from POSIX bytes, Windows UTF-16 units, or
  supported ASCII storage;
- `normalize`;
- `join`, `push`, `reserve_path`, `set_file_name`, and `set_extension`;
- component views: `parent`, `file_name`, `file_stem`, `extension`,
  `components`, and `next_component`;
- predicates: `is_absolute`, `is_relative`, `has_root`, `starts_with`, and
  `ends_with`.

Paths are not strings by accident. Preserve their platform representation and
use typed constructors. Do not manually concatenate host path separators in
protocol or filesystem code.

## 21. Build manifests and runtime profiles

The root manifest has this general shape:

```toml
[package]
name = "my_program"
version = "0.1.0"
edition = "alpha"
entry = "main"

[build]
target = "host"
profile = "debug"
runtime = "std"
native_module = "actus"
position_independent = true

[dependencies]
```

Use `runtime = "core"` or omit the standard-library opt-in when the package
must remain dependency-free. Use `runtime = "std"` for compiler-owned hosted
I/O, filesystem, and path modules.

Dependencies use manifest-relative package paths:

```toml
[dependencies]
io = { path = "library/std" }
fs = { path = "library/std" }
path = { path = "library/std" }
```

Run `actus lock` after changing dependencies. `actus lock --check` verifies
that `Actus.lock` agrees with the manifest and package checksums.

The current hosted target is `host`. Target profiles are compiler/toolchain
configuration, not hard-coded language types. Future ARM Cortex, RISC-V,
STM32, RP, WASM, or other profiles must be added through target manifests and
backend/runtime contracts rather than by changing the meaning of `u32`,
`Buffer`, or `pack` in source.

## 22. CLI workflow for an agent

### Create and initialize

```sh
actus new my_program
actus new my_program --runtime std
actus init --no-git
```

`new` creates a project directory. `init` initializes an existing directory.
Neither command should be assumed to compile or publish code.

### Validate and build

```sh
actus check
actus check --strict
actus build --emit obj -o target/program.o
actus build --emit exe -o target/program
actus build --strict --emit exe -o target/program
actus run
```

`check` validates source without producing a final native artifact. `build`
emits an object or executable. `run` builds a temporary executable, runs it,
returns its exit code, and removes the temporary executable.

### Tests, formatting, lockfile, and watch

```sh
actus test
actus test --strict
actus fmt --check
actus fmt
actus lock
actus lock --check
actus watch --once
```

`--strict` is a release/conformance validation mode for supported `check`,
`build`, and `test` workflows. It validates fail-closed source, ownership,
architecture, documentation, target, lockfile, and execution contracts.
It is not an argument for `run`, `fmt`, or `watch`.

### Repository development commands

When working inside this Rust compiler checkout, use:

```sh
cargo run --bin actus -- check examples/hello.act --strict
cargo run --bin actus -- test --strict
```

The Cargo form is contributor/bootstrap workflow. It is not the end-user
installation model.

### Current systems-language capability status

The current systems-language capability set is compiler-owned and
target-neutral; CPU identities belong in target and runtime configuration,
not in Actus syntax.

#### Const generics

The initial `Usize` const-generic slice works across parsing, semantic
resolution, bounded array capacity, aggregate layout, generic identity, native
specialization, and strict/native acceptance. Use literal arguments such as
`Fabric[8]`. Do not use runtime values or assume arbitrary const expressions
are implemented.

Const parameters may also be used as type arguments in generic verb
signatures and bodies, and their specialized values may be read in the verb
body:

```act
struct Storage[N: Usize] {
    erg values: Array[u32, N],
}

verb clear[N: Usize](ins storage: Storage[N]) {
    storage.values[0] = 42u32;
}

verb capacity[N: Usize](ins storage: Storage[N]) -> u32 {
    if 0u32 < (N as u32) {
        return N as u32;
    }
    return 0u32;
}

verb main() -> Int {
    erg storage: Storage[4] = Storage[4] {
        values: Array[u32, 4](),
    };
    clear(storage: ins storage);
    return capacity(storage: ins storage) as Int;
}
```

The const argument must be a positive compile-time `Usize` literal or a
declared const parameter. Runtime expressions, zero capacities, reference
roles, and nested arguments remain invalid. `Buffer` is a compiler-owned
builtin type name; use a domain-specific name such as `Storage` for a user
defined generic aggregate.

Generic verbs may call other generic verbs without manually materializing the
helper. The compiler propagates the concrete caller substitution transitively:

```act
verb inner[N: Usize](ins storage: Storage[N]) -> u32 {
    return N as u32;
}

verb outer[N: Usize](ins storage: Storage[N]) -> u32 {
    return inner(storage: ins storage);
}
```

When `outer` is reached as `outer[4]`, the compiler also materializes the
reachable `inner[4]` instance before native declaration and lowering. This
propagation is deterministic and deduplicated; do not add duplicate helper
verbs or handwritten concrete wrappers.

When a specialized generic verb has an aggregate parameter such as
`Storage[256]`, the complete concrete aggregate identity is retained through
native layout and ABI planning. This includes aggregates containing bounded
arrays of packed elements. The same contract applies to `ins` and `abs`
parameters, direct concrete calls, facade imports, and nested generic calls.
Typed aggregate locals inside the specialized verb receive the caller's const
substitution before native lowering; source-level concrete wrapper functions
are not required.

Each concrete generic call site remains part of the specialization record even
when several call sites use the same canonical instance. The compiler may
reuse one native specialization for those calls, but it must retain every
source span needed to rewrite each call before native dependency collection.
Consequently, two calls such as `read_capacity[4]()` in one native call graph
must both lower to the specialized symbol. A generic-instance cache must not
deduplicate distinct call sites solely by canonical type arguments and caller.

Const generic parameters are also valid read-only compile-time values inside
case guards. They may be used directly or through a cast and are resolved
before native lowering:

```act
verb choose[N: Usize]() -> u32 {
    return case true {
        true if 0u32 < (N as u32) => N as u32,
        _ => 0u32,
    };
}
```

The compiler specializes `choose[4]` with the concrete value `4`; `N` is not
a mutable runtime binding and cannot be assigned, borrowed, or transferred.
Case-guard access must remain deterministic across direct and transitively
specialized generic verbs.

#### Array return ABI

Returning `Array[T, N]` uses the same caller-owned return-slot ABI as other
indirect aggregate values. The caller allocates the destination slot, the
callee copies the complete bounded array into it, and the caller may then
index the returned value normally:

```act
verb make_values() -> Array[u32, 2] {
    erg values: Array[u32, 2] = Array[u32, 2]();
    values[0] = 41u32;
    values[1] = 42u32;
    return values;
}

verb main() -> Int {
    erg values: Array[u32, 2] = make_values();
    return values[0] as Int + values[1] as Int;
}
```

This prevents a returned array from referring to callee-local stack storage.
The ABI contract covers signature generation, call lowering, caller slot
allocation, callee copy-out, cleanup, and indexed access. Validate changes
with both small arrays and systems-sized arrays such as `Array[u32, 8]`.

#### Boolean literal expressions

`true` and `false` are first-class `Bool` expressions in local initializers,
returns, call arguments, aggregate fields, array elements, conditions, case
expressions, nested blocks, and value-producing branches:

```act
verb accepts_packet(abs packet: Buffer) -> Bool {
    erg linked: Bool = false;
    if linked || length(packet) > 0 {
        return true;
    }
    return false;
}
```

Boolean literals retain their source spans for diagnostics, formatter output,
hover, semantic tokens, and the LSP semantic model. They do not coerce to
integers. `&&` and `||` remain short-circuiting and require `Bool` operands.

#### Nested places, calls, and control flow

The parser and AST represent complete writable place chains without losing
selector order:

```act
struct Column {
    erg axon_0: u32,
}

struct Fabric {
    erg columns: Array[Column, 2],
}

verb notify(erg value: u32) -> Void {
    return;
}

verb update(erg fabric: Fabric, erg index: Usize, erg slot: u32) -> Void {
    if index == 0 {
        fabric.columns[index].axon_0 = slot;
        notify(value: fabric.columns[index].axon_0);
    }
    return;
}
```

Supported and tested forms include identifier assignment, field assignment,
indexed assignment, mixed index/field chains, nested indexed aggregate places,
calls inside nested statement blocks, all ownership argument roles, `?`
propagation from nested calls, nested `if`/`else` expressions, typed branch
joins, and diverging branches such as `return`, `break`, and `continue`.
The left-hand place of a compound assignment is evaluated exactly once; native
lowering follows address calculation, load, operation, and store through that
same place.

Case subjects may also be readable aggregate places, including a struct field
or an indexed element:

```act
enum State { Ready, Busy, }
struct Slot { state: State, }

verb inspect(erg slot: Slot) -> Int {
    return case slot.state {
        State.Ready => 1,
        State.Busy => 2,
    };
}
```

The semantic analyzer resolves the root binding of the place and applies the
case borrow or ownership rule to that binding. `case abs` remains read-only,
and `case dat` still requires an owned movable subject. A field or indexed
place does not bypass borrow tracking or turn a temporary expression into an
owner.

#### Package-aware test execution

`actus test` analyzes test sources through the package module graph. When a
canonical facade exists, the test file is compiled as part of that aggregate,
so sibling declarations are visible in the same way they are for `actus
check` and `actus build`.

When adding or moving tests:

- keep the test in a source tree discovered by the package manifest;
- expose sibling declarations through the canonical directory facade;
- do not make a test pass by parsing the test file as an isolated module;
- collect test metadata from the original source while compiling the complete
  module aggregate;
- verify both test discovery and package-visible type resolution.

This is required for tests that refer to declarations such as
`CorticalFabric` or `Array[Minicolumn, 64]` from sibling modules.

#### Typed native control-flow joins

An `if` or `case` branch that produces a value must preserve its declared
native type through every nested block. A fixed-width value such as `u32` must
not become `Int` merely because it passed through a branch, call result, index
expression, or native merge block.

Apply these rules when writing or reviewing control flow:

- value-producing branches must have compatible native types;
- `return`, `break`, `continue`, and typed `?` paths are diverging paths and
  do not participate as ordinary values in a join;
- branch-local assignments to an existing owner must be represented by a
  native merge value, not discarded branch-local state;
- the same owner must be cleaned exactly once after a merge or direct owned
  return;
- a semicolon-terminated final expression is value-producing only where the
  enclosing expression grammar explicitly permits it.

For indexed native code, preserve this sequence:

```text
evaluate base and index once -> bounds check -> calculate address -> load/store
```

Never recalculate a side-effecting base or index for a later field access. The
rule applies to ordinary arrays, pack arrays, nested structs, and array-backed
pack storage.

#### Native IR verification

When a workload promises integer-only or zero-floating-point execution, inspect
the generated Cranelift IR rather than relying on source appearance or object
byte-pattern guesses. A valid audit checks:

- instruction opcodes for floating-point operations and constants;
- operand types, including floating-point comparison operands whose result is
  an integer condition;
- result types, including floating-point vector lanes where applicable;
- one integer-only fixture and one float-positive fixture.

The compiler owns this audit at the native lowering boundary. Backend-focused
tests can enable it with the native configuration builder:

```rust
let configuration = NativeBackendConfiguration::default()
    .with_no_float_ir_verification();
```

The default remains disabled so existing floating-point native workloads keep
their documented behavior. With the option enabled, every generated native
function is inspected after Actus lowering and before object emission. A
failure identifies the owning function and the detected floating-point IR
instruction identities; it is not inferred from disassembly or byte-pattern
heuristics. The compiler-level evidence is covered by
`test_zero_float_verification_inspects_generated_integer_ir`,
`zero_float_verification_rejects_generated_float_ir`, and the unit audit tests
in `src/codegen/native/ir_audit.rs`.

For a package-wide contract, configure the same compiler-owned audit in the
manifest rather than enabling it only in an individual backend test:

```toml
[build]
verify_no_float_ir = true
```

This package policy is forwarded to every Actus-generated native object unit,
including module, generic, and performance specializations. The acceptance
workflow should run `check --strict` and `test --strict` first, then build both
an object and an executable. `check` validates source and semantics but does
not itself generate IR; the zero-float claim is established by the native
build report before object emission. The package-level evidence is covered by
the positive and negative zero-float CLI workflow tests in
`tests/cli_workflow.rs`.

The contract covers generated Actus IR only. Separately linked runtime
objects, external ABI implementations, compiler intrinsics, and platform
libraries require their own audit and must not be presented as covered by
`verify_no_float_ir`.

#### Array-backed packed storage

The compiler supports bounded byte-array storage for hardware and binary-layout
packs. The production-supported form is `Array[u8, N]`; it is a distinct pack
layout, not an ordinary array workaround. The capacity is compile-time bounded,
the storage is inline, and the pack's total width is `N * 8` bits.

The following 64-byte/512-bit shape is accepted and natively lowered:

```act
pack Minicolumn {
    erg storage: Array[u8, 64];
    layout little;
    fields {
        erg charge: u8 at 0;
        erg threshold: u8 at 8;
        erg coincidence_low: u64 at 64;
        erg coincidence_high: u64 at 128;
        erg axon_0: u32 at 192;
        erg axon_7: u32 at 416;
        erg inhibitory_link: u32 at 480;
    }
}
```

The complete readiness fixture contains the full 512-bit field map:
`charge`, `threshold`, `myelination`, `idle_ticks`, `flags`, `payload`,
`layer_depth`, both coincidence words, eight axon targets,
`free_list_link`, and `inhibitory_link`. Field offsets remain explicit and
must cover the storage contract without overlap or uncovered bits.

Array-backed packs provide:

- checked indexed byte reads, writes, and compound assignments;
- bit-packed field reads/writes with masking, shifting, width validation, and
  little-endian mapping;
- fixed-width `u8`, `u16`, `u32`, and `u64` field arithmetic;
- copy, move, return, aggregate initialization, and bounded inline layout;
- `erg`, `abs`, and `ins` ownership behavior through indexed storage places;
- deterministic bounds traps for invalid runtime indexes and compile-time
  diagnostics for statically impossible indexes;
- Buffer snapshot/restore through ordinary Actus operations without a hidden
  heap allocation;
- native executable and object emission without a C serialization bridge.

#### Indexed fields inside packed values

A packed value may contain one bounded, fixed-width array field when the field
is fully covered by the declared storage:

```act
pack Example {
    erg storage: Array[u8, 32];
    layout little;
    fields {
        erg links: Array[u32, 8] at 0;
    }
}
```

The array field occupies `element_width * count` bits beginning at its
declared base offset. It is not a dynamic collection, slice, pointer, or
separate allocation. The compiler records the element type, fixed count,
total width, offset, and ownership role in semantic and LSP metadata.

Use the ordinary checked indexing syntax for reads, writes, and compound
updates:

```act
abs value: u32 = example.links[index];
example.links[index] = next_value;
example.links[index] += 1u32;
```

Constant indexes outside `[0, count)` are rejected during semantic analysis;
runtime indexes use the normal checked bounds path. Immutable pack views
cannot be mutated. Indexed fields compose with arrays of pack values, so
`cells[slot].links[index]` remains an ordinary bounded place expression.

Native indexed pack fields require byte-addressable storage, a byte-aligned
base offset, and an element width that is a whole number of bytes. Little and
big endian layouts use the declared byte order for element loads and stores.
Unsupported representations fail with a compiler diagnostic rather than
falling back to unchecked pointer arithmetic. Existing scalar bit-packed
fields keep their masking and shifting lowering.

For dynamic indexed byte loops, use a supported integer index such as `u32`:

```act
verb snapshot(ins output: Buffer, abs column: Minicolumn) -> Int {
    erg index: u32 = 0;
    loop {
        if index >= 64 {
            break;
        }
        append(output, column.storage[index]);
        index += 1;
    }
    return index as Int;
}
```

`Usize` remains the tested const-generic domain for declarations such as
`Array[T, N]`. Do not silently substitute runtime capacity, `Array[u16, N]`,
or an unbounded storage representation for the supported packed-storage
contract.

#### Explicit scalar reuse

Use `copy(value: abs value)` when an eligible scalar must be reused at an
owning call boundary. The compiler accepts `Int`, `Bool`, and fixed-width
integer values. The `abs` role is explicit for this intrinsic and the operation
does not consume the caller. Do not use identity arithmetic such as `value + 0u32` to express
reuse. Buffers, strings, resources, cleanup-bearing aggregates, and
unsupported user-defined values are rejected with `E1021`; a missing explicit
`abs` role or a missing safe inferred role is rejected with `E1016`.

Static local calls support a narrow ownership inference profile. When a call
argument omits its role, the compiler may infer `abs` only from an `abs`
binding and `ins` only from an `ins` binding that is active and mutable. The
inference is resolved before ownership validation and native lowering. It does
not apply to `erg` or `dat`, dynamic or external calls, aggregates, buffers,
resources, or ambiguous expressions. Explicit `abs value` and `ins value`
remain valid and are the source-level opt-out when the ownership intent should
be visible. An unsafe omission uses the stable `E1016` argument-role diagnostic.

For a direct static call argument, the compiler may materialize a hidden
`erg` local when the source expression is a pure scalar expression and the
argument explicitly uses `abs`. This keeps small arithmetic and bounded index
expressions readable without changing ownership or evaluation order:

```act
verb read(abs value: u32) -> Int {
    return value as Int;
}

verb main() -> Int {
    erg values: Array[u32, 2] = Array[u32, 2]();
    values[1] = 41u32;
    erg index: u32 = 1u32;
    return read(value: abs values[index]);
}
```

The compiler-owned local is equivalent to an explicit scalar binding for the
duration of that call. It is generated in deterministic source order, receives
`erg`, and is passed to the callee as `abs`. Generated names are reserved
internally and avoid collisions with source bindings. Formatter and LSP
semantic declarations do not display the hidden local. The initializer may
contain scalar literals, arithmetic, bitwise operations, comparisons, casts,
grouping, and bounded scalar indexing. Calls, method calls, buffers,
aggregates, resources, mutation, borrows, conditional expressions, and
ownership transfers remain explicit source forms.

Validated scalar constants are handled by the same normalization boundary when
they are passed with an explicit `abs` role. The compiler materializes the
constant as a hidden scalar `erg` local before native constant inlining, then
passes that local as `abs`. This preserves the source ownership contract and
prevents native preparation from turning `abs CONSTANT` into an invalid
borrow of a literal (`E1016`). Constants are not generalized into runtime
storage, and non-scalar or resource constants remain outside this rule.

The generated form has the same native text as its equivalent explicit local,
and its object relocations contain no allocation reference. A prior move or
borrow rule is still diagnosed by ordinary semantic analysis; normalization
does not repair invalid ownership.

When an imported generic verb named `copy` is visible, dispatch is determined
by call shape and declaration provenance. A local module-scoped `copy`
declaration has precedence. The one-argument scalar form
`copy(value: abs scalar)` uses the compiler-checked scalar reuse intrinsic when
no local declaration shadows it. The two-argument `reader`/`writer` form uses
the imported generic standard-library verb. Generic specialization must occur
only after this dispatch decision so unresolved parameters cannot reach native
semantic reanalysis. This rule does not relax the explicit `abs` requirement
or the supported scalar type set.

Indexed selection remains bounded through `values[index]`. Constant indexes
outside `0 <= index < N` produce `E1085`; computed indexes retain the native
bounds check. Indexed views preserve the source ownership role, so an
exclusive `ins` view cannot be created from an `abs` owner (`E1016`).

The formatter emits array-backed packs in canonical multiline form while
preserving storage type, field offsets, layout, and `"""` documentation. The
LSP exposes storage width, byte capacity, layout, field metadata, hover,
completion, semantic tokens, definition/navigation, rename, and overlay
reindexing for these packs. Pack diagnostics retain stable codes and exact
source spans, including invalid storage (`E1070`), out-of-range fields
(`E1072`), overlap (`E1073`), constant index bounds (`E1085`), and ownership
violations (`E1051`).

The canonical end-to-end evidence is maintained in:

```text
the repository's strict package and executable acceptance tests
```

That acceptance runs strict check/test/format validation, host-native build and
execution on `x86_64-unknown-linux-gnu` with exit status `126`, object emission
with `.text` and `main`, and rejected fixtures for overlap, field bounds,
invalid storage elements, runtime capacity, indexed bounds, and ownership.
Do not claim a packed-storage feature is complete from parsing alone; require
semantic, native, formatter, LSP, rejected-fixture, and executable evidence.

#### Pack values as array elements

Declared `pack` types are also valid typed elements of bounded arrays. The
supported form is `Array[Pack, N]`, where every element is stored inline using
the pack's complete native layout and stride. A pack is not lowered as a
pointer, opaque handle, or manually flattened byte array.

```act
pack Cell {
    erg storage: Array[u8, 2];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg tail: u8 at 8;
    }
}

verb main() -> Int {
    erg cells: Array[Cell, 2] = Array[Cell, 2]();
    cells[1] = Cell { storage: Array[u8, 2](), };
    cells[1].marker = 41u8;
    cells[1].marker += 1u8;
    return cells[1].marker as Int;
}
```

The compiler registers pack declarations before validating struct and array
references. Array layout resolution preserves pack identity, field metadata,
alignment, size, and the complete inline element stride. Dependent storage
arrays are laid out before arrays containing those packs, so an
`Array[Minicolumn, 64]` layout can use a 64-byte array-backed `Minicolumn`
without a runtime descriptor.

Pack-array places support:

- checked indexed element access and field access such as `cells[index].marker`;
- field assignment and compound assignment through the same checked address;
- one evaluation of the array base and index before native address lowering;
- `erg`, `abs`, and `ins` ownership behavior, including an `ins` loan of one
  pack element that restores the original array owner;
- aggregate initialization, copy/move/return paths, and cleanup boundaries;
- deterministic rejection of unknown pack types, invalid layouts, impossible
  indexes, illegal aliases, use-after-move, and layout/stride overflow.

The corresponding evidence is:

```text
tests/semantic/packs.rs::accepts_pack_types_as_bounded_array_elements
tests/semantic/packs.rs::rejects_unknown_pack_types_as_array_elements
tests/arrays_cli.rs::executes_array_of_array_backed_packs_natively
tests/arrays_cli.rs::executes_compound_assignment_through_array_of_packs_natively
tests/arrays_cli.rs::preserves_ins_loan_through_an_array_of_packs_natively
tests/formatter/suite.rs::formats_pack_array_element_types_idempotently
tests/lsp/semantic_intelligence.rs::lsp_accepts_pack_types_as_array_element_types
```

This capability is complete only when these semantic, native, formatter, and
LSP checks remain green together with strict checking, object emission, source
limits, documentation checks, and the full repository test suite.

#### Formatter and LSP parity

Implemented syntax is complete only when the compiler and tooling agree. The
formatter preserves Actus `"""` documentation strings, imports, declaration
order, selector structure, and reparsability. The LSP understands const
generic parameters, Boolean literals, nested fields/places, diagnostics,
hover, definitions, semantic tokens, formatting, versioned overlays, and
malformed nested documents without process termination.

#### Verification boundary

The verification workflow covers the combined systems-language contract:

- strict package checking succeeds;
- accepted and rejected ownership cases are tested;
- formatting check succeeds without moving or deleting documentation;
- native execution succeeds;
- LSP analysis remains synchronized with the workspace source;
- source limits, public documentation, and native IR verification checks pass.

This status describes the implemented compiler capabilities and their required
verification evidence. Deferred language features require separate designs,
implementations, and tests before they may be used.

## LSP and editor behavior

The compiler-backed LSP supports a workspace/document model with versioned
overlays and module-aware analysis. An agent changing language syntax must
consider all of the following, not only compiler acceptance:

- diagnostics and source spans;
- hover and semantic model data;
- completion and signature help;
- definition/navigation;
- rename and open-document overlays;
- semantic tokens;
- code lenses;
- formatting and idempotence;
- cancellation, stale versions, and invalid ranges;
- URI normalization across platforms;
- malformed JSON-RPC and missing documents without process termination.

Unsaved source must be analyzed consistently with the current in-memory
workspace. A new syntax is not complete if the LSP treats it as invalid,
reorders its docstrings, or reports stale module interfaces.

## 24. Formatter rules

The formatter is a source-preserving language tool, not a minifier. It may
normalize indentation and spacing, but it must preserve:

- triple-quoted Actus documentation strings;
- imports and module facade declarations;
- declaration order and semantic boundaries;
- comments/docstrings attached to the declaration they describe;
- all tokens required for a reparsable program.

Formatting must be idempotent: formatting an already formatted file produces
the same file. Always run `actus fmt --check` after formatting and then run
`actus check` on the result. If formatting moves or deletes documentation,
imports, or declarations, treat it as a compiler bug and add a regression
test rather than accepting the output.

## 25. Diagnostics and error design

Diagnostics have stable codes, source spans, human-readable messages, and a
structured representation independent of terminal rendering. When adding a
new diagnostic:

1. assign a stable `E####` code;
2. identify the exact source span;
3. explain the violated language contract;
4. provide a correction when it is unambiguous;
5. add a positive/negative regression test;
6. verify CLI, LSP, and formatted rendering;
7. ensure malformed input does not panic or terminate the compiler.

A parser may reject malformed syntax. Semantic analysis must reject invalid
types, ownership, module visibility, bounds, constants, and ABI contracts.
Code generation must report an internal boundary error rather than inventing a
semantic repair.

## 26. Native and embedded-oriented design rules

Actus is intended to make low-level work explicit without sacrificing the
compiler's ownership model. For embedded or protocol code:

- use fixed-width integers for wire/register fields;
- use `pack` for explicit layouts and masks;
- use `Array[T, N]` for bounded static storage;
- use caller-owned `ins Buffer` storage for zero-copy mutation;
- use `abs Buffer` for read-only inspection;
- use `dat` when ownership really ends at the callee;
- isolate MMIO and foreign calls behind `unsafe extern` facades;
- document endianness, alignment, register width, and status contracts;
- keep target/ABI details in build/runtime configuration;
- test both hosted behavior and freestanding/object boundaries where relevant.

Do not claim that the current host target is already a complete bare-metal
target. ARM, Cortex-M, Cortex-A, RISC-V, STM32, RP, and other profiles need
linker layout, interrupt ABI, register/atomic capabilities, memory regions,
runtime profile, and standard-library support outside the language syntax.

## 27. What is not currently safe to assume

The following require separate evidence and must not be invented in examples:

- unbounded `for`, `while`, or general iterator syntax;
- async/await, tasks, actors, closures, lambdas, macros, or REPL;
- automatic numeric promotion or implicit casts;
- null values or nullable references;
- an unbounded heap or garbage collection;
- stable cross-platform ABI for every aggregate;
- every target-specific runtime profile;
- WASM, DWARF, incremental compilation, or parallel compilation;
- a complete `Map` standard-library API;
- a protocol or domain runtime merely because Actus can express packs, arrays,
  buffers, and fixed-width arithmetic.

If a requested feature falls into this list, report it as a language/toolchain
gap and propose the parser, AST, semantic, codegen, tooling, and test work
needed to implement it. Do not hide the gap behind an unsafe bridge or a
handwritten special case.

## 28. Recommended coding patterns

### Typed failure instead of sentinel status

```act
verb parse_length(abs frame: Buffer) -> Result[u16, DecodeError] {
    if length(frame) < HEADER_SIZE {
        return Result[u16, DecodeError].Err(DecodeError.ShortFrame);
    }
    return Result[u16, DecodeError].Ok(read_length(frame: abs frame));
}
```

Prefer `Result` at public boundaries. Translate foreign negative statuses once
and preserve the typed error after that.

### Explicit ownership in a pipeline

```act
verb process(ins storage: Buffer) -> Result[Int, IoError] {
    erg bytes = read_line(buffer: ins storage)?;
    inspect(input: abs bytes)?;
    return write(buffer: abs bytes);
}
```

Make every transfer visible at the call site. Avoid hidden copies and do not
pass an owner as an ordinary argument when the callee needs a different role.

### Fixed-width protocol arithmetic

```act
const HEADER_SIZE: u16 = 8u16;
const FLAG_VALID: u8 = 0x01u8;

verb valid_flags(erg flags: u8) -> Bool {
    return (flags & FLAG_VALID) != 0u8;
}
```

Use typed literals and constants. Avoid `Int` for fields whose width is part of
the wire or hardware contract.

## 29. Agent workflow checklist

Before editing:

- read `AGENTS.md` and this guide;
- inspect `git status` and confirm the repository boundary;
- locate the canonical source, facade, tests, and docs for the feature;
- classify the requested behavior as implemented, bounded, designed, or new;
- check file/function size and module ownership.

While editing:

- keep the compiler pipeline one-directional;
- preserve ownership role spelling and explicit call-site markers;
- keep unsafe bridges private and typed facades public;
- preserve Actus `"""` documentation strings;
- add accepted and rejected tests;
- update LSP/formatter/diagnostic behavior for syntax changes;
- do not add a target-specific language special case;
- do not mark planned work complete without direct evidence.

Before handoff:

```sh
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
scripts/check_source_limits.sh
git diff --check
```

For Actus source or standard-library changes, also run the relevant `actus`
check/build/test commands and native execution tests. For documentation-only
changes, run the documentation consistency checks and still inspect links and
claims manually.

## 30. Reference map

Use these repository documents as focused references. The canonical Actus
repository root is:

```text
/home/magradze/Projects/actus_project/actus
```

The paths below are absolute on the development machine so an agent launched
from another project directory can locate the source of truth directly:

- `/home/magradze/Projects/actus_project/actus/README.md`: project status, build basics, runtime selection, and ABI limits.
- `/home/magradze/Projects/actus_project/actus/docs/language/alpha-user-guide.md`: implemented Alpha workflow.
- `/home/magradze/Projects/actus_project/actus/docs/language/lexical-map.md`: keywords and future/reserved vocabulary.
- `/home/magradze/Projects/actus_project/actus/docs/language/operators.md`: operator precedence and operand contracts.
- `/home/magradze/Projects/actus_project/actus/docs/language/alpha-guarantees.md`: ownership, borrowing, cleanup, and
  bounded storage guarantees.
- `/home/magradze/Projects/actus_project/actus/docs/language/style-and-conventions.md`: source style and naming.
- `/home/magradze/Projects/actus_project/actus/docs/language/lsp-protocol.md`: editor/LSP contract.
- `/home/magradze/Projects/actus_project/actus/docs/conformance/limitless-policy.md`: source-limit exceptions.
- `/home/magradze/Projects/actus_project/actus/docs/architecture/compiler-pipeline.md`: compiler architecture.
- `/home/magradze/Projects/actus_project/actus/docs/decisions/ADR-0052-core-control-flow-constants-and-type-directed-ergonomics.md`:
  current ergonomic/core direction.
- `/home/magradze/Projects/actus_project/actus/docs/decisions/ADR-0053-production-language-capability-and-wire-readiness.md`:
  production capability boundary.
- `/home/magradze/Projects/actus_project/actus/examples/`: executable language examples.
- `/home/magradze/Projects/actus_project/actus/library/std/src/`: public standard-library facades and sibling modules.
- `/home/magradze/Projects/actus_project/actus/tests/`: compiler, native, runtime, LSP, and standard-library evidence.

The source code and tests are authoritative when a prose document is stale.
If this guide and the compiler disagree, do not silently choose one: report
the inconsistency, update the guide and focused documentation, and add or
repair the test that defines the intended behavior.
