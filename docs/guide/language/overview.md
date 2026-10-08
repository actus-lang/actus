# Actus language overview

Actus is a systems programming language where types, ownership, bounded storage,
and native output are described directly in source code.

A program passes through these stages:

```text
source -> lexer -> parser -> AST -> semantic analysis -> ownership and cleanup
       -> native lowering -> object or executable output
```

Each stage has one job. The lexer reads characters, the parser builds the
source structure, semantic analysis checks types and ownership, cleanup plans
scope destruction, and native lowering produces target code. A later stage
must not repair an invalid earlier stage.

## The three ideas used in every program

### Explicit ownership

Every value has an ownership meaning. Actus uses roles such as `erg`, `abs`,
`dat`, and `ins` to say whether a binding owns a value, views it, transfers it,
or lends it mutably for one call.

### Bounded storage

Arrays, buffers, packs, arenas, parser state, and protocol data have explicit
bounds. Access outside a bound is rejected or returns a typed failure according
to the API contract.

### Typed failure

Operations that can fail return a declared type such as `Result[T, E]`. A
caller handles that result with `case` or propagates it with `?` where the
surrounding verb has a compatible result type.

## Source language and compiler language

Actus source uses `verb`, `erg`, `abs`, `dat`, `ins`, `case`, `pack`, `Array`,
`Buffer`, and other language constructs directly. Compiler commands, package
settings, target profiles, and runtime profiles are configured outside source
in `Actus.toml` and related files.

Keep these layers separate when reading documentation:

- language pages explain what can be written in `.act` files;
- ownership pages explain how values move and are cleaned up;
- module pages explain how files expose declarations;
- standard-library pages explain public APIs;
- compiler pages explain checking and native output;
- workflow pages explain commands and project practice.

## A small complete program

```actus
verb main() -> Int {
    return 0;
}
```

The package configuration selects the entry verb for an executable. In the
current hosted application workflow, that entry is normally `main` and returns
an integer status.

Continue with [the first program](first-program.md) to create a project and
[types and literals](types-and-literals.md) to understand values.
