# Chapter 03: Lexicon and Syntax Fundamentals

A compiler cannot reason about a program until it knows what each character
means and how the resulting pieces fit together. Chapter 02 used a complete
project without opening that layer. This chapter opens it.

We move from source bytes to tokens, from tokens to declarations and
expressions, and from expressions to diagnostics. The current Actus Alpha
syntax is the authority here: Actus uses `verb`, `erg`, `abs`, `dat`, `ins`,
and `case`.

## 1. Lexical Anatomy and Source Encoding

An Actus source file is UTF-8 text. ASCII characters normally occupy one byte;
other Unicode characters may occupy several bytes. The lexer records source
spans as byte offsets, while diagnostics later convert those offsets to
human-readable lines and columns.

For example:

```actus
verb main() -> Int {
    return 0;
}
```

The lexer first identifies tokens:

```text
verb  main  (  )  ->  Int  {  return  0  ;  }
keyword identifier punctuation operator type punctuation keyword integer punctuation punctuation
```

A token stores its kind and a half-open span. The start offset is included and
the end offset is excluded. This lets a diagnostic identify the exact source
region without copying the source text.

### Comments

Actus has ordinary and documentation line comments:

```actus
// This comment is ignored by the compiler.
/// This comment is available to documentation-aware tooling.
verb main() -> Int {
    return 0;
}
```

`//` starts an ordinary comment. `///` starts a documentation comment. Both
continue to the end of the line and neither changes the grammar of the next
declaration. The LSP can use documentation comments in hover information.

### Whitespace and semicolons

Spaces, tabs, and line breaks normally separate tokens without changing their
meaning:

```actus
verb one() -> Int { return 0; }

verb two() -> Int {
    return 0;
}
```

The semicolon is meaningful. It terminates statements such as `return`,
bindings, assignments, `drop(name);`, and expression statements. Braces
delimit blocks; they do not remove the semicolon required by a statement.

## 2. Keywords and Reserved Vocabulary

A keyword is a word the lexer recognizes as a language token instead of an
ordinary identifier.

Declaration and module keywords include `verb`, `struct`, `enum`, `role`,
`perform`, `import`, and `open`. They introduce actions, data types,
contracts, implementations, modules, and facade exports.

Ownership and low-level boundary keywords include `erg`, `abs`, `dat`, `ins`,
`ref`, `drop`, `unsafe`, and `extern`. `erg`, `abs`, `dat`, and `ins` describe
value roles. `ref` constructs a borrow expression for an `abs` binding or expression;
it does not create an owner or a raw pointer. `drop` is a dedicated statement
with the form `drop(name);` that explicitly ends an owned resource.
`unsafe extern "C"` declares a foreign C ABI boundary and keeps that boundary
visible in source.

Control-flow and pattern keywords include `return`, `loop`, `break`,
`continue`, `case`, `if`, and `for`. `return` exits an action. `loop`,
`break`, and `continue` provide the loop control surface. Conditional
branching is structural: Actus uses `case` pattern matching, with boolean
guards written as `Pattern if condition => body`. `if` has meaning inside a
pattern guard; it is not a separate branching construct. `for` is a reserved
iteration word and is not the loop form used by the current language.

Metadata and dispatch keywords include `dynamic` and `meta`. `dynamic` marks
an explicit runtime role-dispatch boundary such as `abs dynamic Writer`.
`meta` introduces compile-time metadata such as `meta test`.

The literals `true` and `false` are boolean tokens. `_` is the wildcard token
used in patterns.

Actus's vocabulary is deliberately small and orthogonal. Actus defines actions
with `verb`, bindings through roles (`erg`, `abs`, `dat`, `ins`), and pattern
branching with `case`.

## 3. Identifiers, Literals, and Primitive Formats

Identifiers name declarations, bindings, parameters, fields, and modules.
They begin with a letter or underscore and may continue with letters, digits,
or underscores. The conventions use `snake_case` for actions, bindings,
parameters, and fields, and `PascalCase` for types, roles, and variants:

```actus
struct Point {
    x_coordinate: F32,
    y_coordinate: F32,
}

verb distance(abs point: Point) -> F32 {
    return point.x_coordinate;
}
```

`Point` is a type name. `x_coordinate` and `y_coordinate` are fields.
`distance` is an action and `point` is its parameter. The parser recognizes
the names; semantic analysis resolves what each name denotes.

### Numeric literals

The current lexer accepts decimal integers and decimal floating literals:

```actus
verb numbers() -> Int {
    erg count = 42;
    return count;
}
```

`42` is an integer token. Semantic analysis checks its type and range. The
built-in integer family includes `Int`, `I8`, `I16`, `I32`, `I64`, `U8`,
`U16`, `U32`, `U64`, and `Usize`.

A decimal point followed by decimal digits creates a floating literal:

```actus
verb coordinate() -> F32 {
    erg x = 1.5;
    return x;
}
```

`1.5` is first a lexical fact. The semantic layer checks whether the context
accepts `F32` or `F64`.

Hexadecimal forms such as `0x2A`, binary forms such as `0b101010`, and
standalone byte-literal syntax are not current scanner forms. They must not be
presented as valid Alpha source until lexer, parser, semantic, and regression
tests support them.

### Boolean and string literals

`true` and `false` are boolean tokens:

```actus
verb enabled() -> Bool {
    return true;
}
```

Double-quoted strings are string literals:

```actus
verb message() -> String {
    return "ready";
}
```

The quotes delimit the value. Escaped quotes do not terminate it. A newline
before the closing quote produces an unterminated-string diagnostic.
Standalone byte literals are not currently implemented.

## 4. Bindings and Mutability

An owned local binding begins with `erg`:

```actus
verb main() -> Int {
    erg value = 42;
    return value;
}
```

`erg` supplies the ownership role. `value` names the binding, `=` initializes
it, and `42` is the initializer expression. The compiler checks its type and
records an active owner in the current scope. If the type owns a resource,
the compiler records the cleanup required when the binding leaves the scope.

An immutable view can be introduced explicitly with `ref`:

```actus
abs view = ref source;
```

Here `ref source` is a borrow expression. It creates a temporary read-only
view of `source`; it does not transfer ownership and it does not produce a raw
pointer. The semantic analyzer checks that the view remains within the owner's
valid scope.

Scalar and resource initialization have the same surface shape but different
semantic consequences:

```actus
verb scalar() -> Int {
    erg first = 10;
    erg second = first;
    return second;
}
```

```actus
verb resource(erg packet: Buffer) -> Int {
    erg local = packet;
    drop(local);
    return 0;
}
```

A scalar may live directly in a register or stack slot. In the resource
example, `local` represents responsibility for the buffer. Initialization
transfers the ownership path from `packet` to `local`; `drop(local)` ends it.
The parser represents `drop(local);` as a dedicated drop statement. The
analyzer rejects later use of a moved or dropped binding and rejects cleanup
that would occur twice.

A binding's spelling does not make it mutable. `erg` supplies exclusive
mutation capability, `abs` supplies read-only access, `dat` transfers
ownership, and `ins` supplies an exclusive loan only at a parameter and call
boundary. `ins` is not a local binding role and cannot be used for a struct
field:

```actus
verb append_marker(ins buffer: Buffer) -> Int {
    append(buffer, 41);
    return 0;
}

verb main() -> Int {
    erg buffer = Buffer[4];
    append_marker(buffer: ins buffer);
    return 0;
}
```

The call-site marker is part of the ownership proof. It makes the temporary
exclusive mutation visible and lets the analyzer suspend the root owner for
exactly the duration of the call.

## 5. Anatomy of a Verb Declaration

The general action form is:

```actus
verb name(role parameter: Type) -> ReturnType {
    // statements
}
```

`verb` begins the declaration. `name` identifies the action. Parentheses hold
the parameters. Each parameter has a role, name, colon, and type. `->` and
`ReturnType` declare the result contract. Braces delimit the body.

A concrete declaration may contain several role-bearing parameters and an
access-qualified return:

```actus
verb transfer(erg target: Buffer, abs view: Buffer, dat item: Item) -> Int {
    return 0;
}

verb sub_view(abs input: Buffer) -> abs Buffer {
    return input.raw_slice(0, 1);
}
```

`target` is exclusive and owned, `view` is read-only, and `item` is consumed.
An `ins` parameter would request a temporary exclusive loan instead of a
permanent transfer. The `abs` before `Buffer` in `-> abs Buffer` says that the
result is a non-owning view. The compiler checks every call against those roles,
types, and origin rules.

Calls use argument labels:

```actus
    erg result = transfer(
        target: dest_buffer,
        view: src_buffer,
        item: item,
    );
```

`target:`, `view:`, and `item:` select parameters; they are not new variables.
The parser stores labels in the call AST. Semantic analysis verifies that each
label exists, occurs once, and receives a compatible argument.

At the machine level, scalar arguments may use registers according to the
target ABI. Larger values and resource handles may use addresses and metadata.
The role is a compile-time contract, not necessarily an additional runtime
word.

## 6. Expressions and Statements

An expression produces a value: a literal, binding reference, call,
arithmetic operation, struct literal, or `case` expression. A statement
performs an action in a block: a binding, assignment, `drop`, loop control, or
`return`.

```actus
verb calculate() -> Int {
    erg base = 20;
    return base + 22;
}
```

`base + 22` is the returned expression. `return` transfers control and `;`
terminates the statement.

The current loop form is `loop`:

```actus
verb countdown() -> Int {
    erg value = 3;
    loop {
        value = value - 1;
        break;
    }
    return value;
}
```

The block after `loop` is a statement body. `break` exits it and `continue`
skips to the next iteration where permitted. Conditional branching is handled
by structural pattern matching:

```actus
return case color {
    Color.Red if enabled => 1,
    _ => 0,
};
```

The guard must produce `Bool` and can inspect bindings read-only. Each `case`
branch contains a pattern, `=>`, and a body expression or block according to
the parser grammar.

## 7. Lexer and Parser Diagnostics

A lexical diagnostic means that the character stream contains something the
lexer cannot classify. This source contains an invalid `@`:

```actus
verb main() -> Int {
    return @;
}
```

The lexer reports it before parsing:

```text
error[E0001] at 2:12: unexpected character `@`
```

`E0001` identifies an unexpected-character error. The one-based location
identifies the offending character.

An unterminated string has the same lexical boundary problem:

```actus
verb message() -> String {
    return "ready;
}
```

The scanner reports the missing closing quote rather than producing an
incomplete token.

A parser diagnostic means that lexing succeeded but the tokens do not form a
grammar construct. Here the parameter colon is missing:

```actus
verb main(erg value Int) -> Int {
    return value;
}
```

The parser expects `:` after `value`:

```text
error[E0003] at 1:22: expected `:`, found Identifier("Int")
```

The parser knows the expected grammar element, found token, and source span.
It does not guess the programmer's intention.

The stage boundaries are deliberate:

```text
characters ──lexer──> tokens ──parser──> AST ──semantic analyzer──> proof
```

A lexer error is not repaired by semantic analysis, and a parser error never
reaches Cranelift.

## 8. Summary and the Type-System Bridge

Actus source begins as UTF-8 text. The lexer recognizes comments, whitespace,
keywords, identifiers, literals, operators, and delimiters. The parser turns
those tokens into declarations, statements, expressions, and source spans.

The current lexical contract is explicit:

- actions use `verb`;
- ownership and access use `erg`, `abs`, `dat`, and `ins`;
- non-owning returned views use an access-qualified return such as `-> abs Buffer`;
- pattern branching uses `case`;
- statements end with semicolons;
- hexadecimal and binary integer literals, and byte literals, are outside the
  current literal grammar.

Chapter 04 moves from names and grammar to values. It explains primitive types,
`Buffer`, `struct`, `enum`, compiler-provided `Option[T]` and `Result[T, E]`,
and the layout decisions that make those types useful in native systems code.
