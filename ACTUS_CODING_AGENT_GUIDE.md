# Actus Coding-Agent Guide

Version: 1.0

This is the operational reference for a coding agent that must read, write,
review, test, or explain Actus source code. It is intentionally more explicit
than a beginner tutorial. An agent should use it as a language contract, an
ownership checklist, a project-workflow guide, and a boundary against
inventing syntax that is only mentioned in a roadmap.

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
3. The behavior is designed in an ADR or roadmap but is not implemented.
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
the implementation does now, not what a roadmap says it may do later.

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

`for`, `while`, `in`, `async`, `await`, `yield`, `spawn`, and actor-related
words are not general implemented control-flow constructs. They must not be
used in new examples unless the relevant implementation has landed. Some are
reserved or planned vocabulary only.

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
an ownership-safe way. `for` and `while` are not substitutes; they are not
currently general loop syntax.

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

### 15.4 `Arena[N]`

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

### 17.3 Visibility

`open` on a declaration or sibling export is the Actus visibility mechanism.
Do not use Rust `pub`, Go `export`, or C header conventions in Actus source.
Fields do not have a separate `pub` keyword; aggregate visibility follows the
declared facade and type contract.

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

The compiler resolves canonical `std::io`, `std::fs`, and `std::path` imports
from the packaged library. Applications must use public typed facade APIs,
not the internal C bridge symbols.

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

### 20.2 `std::fs`

The filesystem facade includes:

- `File` owned file handle;
- `OpenOptions` for read, write, append, truncate, create, and create-new;
- `SeekFrom` and `Seeker`;
- `Metadata` and `MetadataProvider`;
- `file_open`, `file_create`, `file_read`, `file_write`, `file_flush`,
  `file_close`, and file seek operations;
- `read_to_bytes` and `read_to_string`;
- `write_file`;
- `remove_file`, `rename`, `copy_file`, `create_dir`, and `remove_dir`;
- metadata access through paths and file handles.

All public operations return typed `Result` contracts. Files and buffers are
owned resources and must be passed with the correct role. A failed operation
must preserve the documented owner and cleanup behavior.

### 20.3 `std::path`

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

## 23. LSP and editor behavior

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

- general `for`, `while`, or iterator syntax;
- async/await, tasks, actors, closures, lambdas, macros, or REPL;
- automatic numeric promotion or implicit casts;
- null values or nullable references;
- an unbounded heap or garbage collection;
- stable cross-platform ABI for every aggregate;
- every target-specific runtime profile;
- WASM, DWARF, incremental compilation, or parallel compilation;
- a complete `Map` standard-library API;
- Wire or Ustari protocol implementation merely because ADR-0051 exists;
- a production-ready neural or endocrine runtime merely because Actus can
  express packs, arrays, buffers, and fixed-width arithmetic.

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
- do not mark a roadmap checkbox without direct evidence.

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
  production readiness boundary.
- `/home/magradze/Projects/actus_project/actus/docs/roadmap/phase-21-production-language-capability-and-wire-readiness.md`:
  readiness gates and evidence policy.
- `/home/magradze/Projects/actus_project/actus/docs/roadmap/phase-22-enterprise-compiler-platform.md`: deferred enterprise
  compiler-platform work.
- `/home/magradze/Projects/actus_project/actus/examples/`: executable language examples.
- `/home/magradze/Projects/actus_project/actus/library/std/src/`: public standard-library facades and sibling modules.
- `/home/magradze/Projects/actus_project/actus/tests/`: compiler, native, runtime, LSP, and standard-library evidence.

The source code and tests are authoritative when a prose document is stale.
If this guide and the compiler disagree, do not silently choose one: report
the inconsistency, update the guide and focused documentation, and add or
repair the test that defines the intended behavior.
