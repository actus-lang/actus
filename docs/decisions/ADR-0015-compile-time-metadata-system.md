# ADR-0015: Compile-Time Metadata (`meta`) System

- Status: Accepted
- Date: 2026-09-23

## Context

Actus needs a declarative mechanism for compile-time concerns such as memory
layout, ABI and linker metadata, optimization hints, and generated semantic
implementations. These concerns must remain available to system and bare-metal
programs without introducing runtime decorators, wrappers, hidden allocation,
or dynamic closure behavior.

Actus also requires a syntax that fits its own vocabulary. Rust-style
attribute syntax is not adopted, and build configuration must remain separate
from source-level declaration metadata.

## Decision

Actus adopts `meta` as its declaration-level compile-time metadata mechanism.
Metadata is declarative, validated during compilation, and has no implicit
runtime representation or execution behavior.

### Declaration syntax

Metadata appears before the declaration it describes:

```act
meta repr(C), packed
struct Packet {
    erg header: Header,
    payload: U8,
}
```

```act
meta no_mangle, section(".text.boot")
open extern "C" verb reset(): Int;
```

The grammar supports an optional metadata list before supported declarations
and fields:

```text
MetaList ::= MetaDecl+
MetaDecl ::= "meta" MetaItemList
MetaItemList ::= MetaItem ("," MetaItem)*
MetaItem ::= Identifier | Identifier "(" MetaArgumentList? ")"
```

The `meta` keyword is reserved so that metadata boundaries remain explicit and
the grammar can be extended without treating compiler directives as ordinary
identifiers.

### AST representation

Metadata is represented in the AST independently of any backend:

```text
MetaItem {
    name: Identifier,
    arguments: Vec<MetaArgument>,
    span: Span,
}
```

Declarations retain their metadata until semantic analysis validates and
normalizes it. Backend-specific types, Cranelift objects, linker structures,
and target implementation details must not enter the AST.

The semantic layer validates supported targets, argument arity and values,
duplicate metadata, and conflicting metadata. It then produces dedicated
backend-independent models such as `LayoutAttributes`, `LinkageAttributes`,
`OptimizationAttributes`, and `DeriveRequests`.

## Metadata Categories

### Layout

The initial layout metadata is:

```act
meta repr(C)
meta repr(transparent)
meta packed
meta align(16)
```

`repr(C)` and `repr(transparent)` define explicit layout contracts.
`packed` may create unaligned fields and therefore must not silently make
ordinary safe field access unsound. Packed access requires an approved
unaligned operation or an explicit unsafe boundary. Alignment values must be
valid for the target and must satisfy the compiler's alignment constraints.

### Linkage and ABI

Linkage metadata may describe externally visible symbols:

```act
meta no_mangle, section(".text.boot")
open extern "C" verb reset(): Int;
```

`no_mangle` is restricted to intentional external symbols and must not bypass
visibility, ABI, duplicate-symbol, or safety validation. Linker metadata does
not change ownership, borrowing, cleanup, or role semantics.

### Optimization hints

The compiler may support hints such as:

```act
meta inline(always)
verb fast_add(Int left, Int right): Int {
    return left + right;
}
```

Optimization metadata is a request, not a semantic guarantee. A backend may
ignore an optimization hint when target, size, or optimization policy makes it
inappropriate.

### Derivation

Compiler-provided derivations use metadata syntax:

```act
meta derive(Debug, Eq, Hash)
struct Point {
    x: Int,
    y: Int,
}
```

Initial derivations are built into the compiler and generate ordinary Actus
semantic entities. Generated declarations must pass through parsing or an
equivalent AST construction boundary, semantic validation, ownership checks,
and code generation. Derivation cannot bypass language safety rules.

User-defined arbitrary compile-time code generation is outside this decision.

## Separation from Other Configuration

Actus separates compile-time mechanisms by scope:

| Mechanism | Scope | Responsibility |
| --- | --- | --- |
| `meta` | Declaration or field | Layout, ABI, optimization, and derivation metadata |
| `pragma` | Source or module | Explicit source-level compiler directives |
| `Arca.toml` | Project and build | Target, linker, profile, dependency, and build configuration |

Target selection, linker choice, and build profiles belong in `Arca.toml`, not
in declaration metadata. A future `pragma` system must not become a substitute
for project configuration.

## Safety and Semantic Boundaries

Metadata must never:

- disable ownership, borrowing, or deterministic cleanup checks;
- permit borrow escape or invalidate `erg`, `abs`, or `dat` semantics;
- suppress safety diagnostics or create an implicit `unsafe` boundary;
- introduce hidden allocation, wrappers, closures, or runtime decorators;
- execute arbitrary compile-time code in the compiler process;
- inject backend-specific objects into frontend AST nodes.

Attributes that would require such behavior are rejected rather than treated
as unchecked escape hatches.

## Consequences

- Actus gains one extensible and language-specific syntax for declaration
  metadata without adopting Rust attribute notation.
- Parser and AST design account for metadata before later grammar extensions,
  including generics and role declarations.
- Semantic analysis remains responsible for meaning and validation; codegen
  consumes only normalized metadata.
- Low-level layout and ABI control remain available for bare-metal and FFI
  code while ownership and safety rules remain unchanged.
- Derivation can grow incrementally without requiring runtime reflection or a
  general macro system.
- Metadata names and argument contracts must be versioned and documented as
  the compiler evolves.

## Rejected Alternatives

- Rust-style `#[...]` attributes: rejected to preserve Actus syntax and
  terminology.
- Runtime decorators or wrappers: rejected because they conflict with
  zero-cost and deterministic execution goals.
- Backend-specific metadata in the AST: rejected because it violates frontend
  portability and compiler layer boundaries.
- Unrestricted compile-time execution: rejected until a deterministic,
  sandboxed, and separately specified system exists.
