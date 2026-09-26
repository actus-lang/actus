# Chapter 02: First Program and CLI Workflow

Chapter 01 established the central Actus model: a `verb` describes an action,
and `erg`, `abs`, `dat`, and `ins` describe how values participate in it. This chapter
turns that model into a working project.

You will build the reference compiler, create an Arca project, inspect the
files that the project contains, compile a native executable, run it, and read
a compiler diagnostic when the source is invalid. The examples use the
current command-line interface exactly as it exists today. The compiler binary
is named `actus`; Arca is the project, manifest, dependency, and build
workflow managed by that binary.

## 1. Toolchain and Installation

The Actus reference compiler is currently distributed as source code. There is
no separate installer or prebuilt release command assumed by this chapter.
The reference bootstrap compiler is written in Rust and uses Cranelift for
native code generation.

Install a current Rust toolchain with Cargo, then verify it:

```text
$ rustc --version
rustc 1.XX.X (...)
$ cargo --version
cargo 1.XX.X (...)
```

The exact version numbers depend on the machine. `rustc` compiles the
compiler; `cargo` resolves its dependencies, builds its binaries, and runs
its tests. A hosted executable build also needs a system C linker. On Unix
systems this is commonly `cc` or `clang`; on Windows it is supplied by the
selected MSVC or GNU toolchain.

From the Actus repository, build the compiler in release mode:

```sh
cargo build --release --bin actus
```

Cargo places the executable at:

```text
target/release/actus
```

You can run it through Cargo while developing the compiler:

```sh
cargo run --bin actus -- --help
```

Or place `target/release` on your `PATH` and invoke the installed binary:

```sh
actus --help
```

The command after `--` belongs to the Actus binary rather than to Cargo.
For example, in `cargo run --bin actus -- build ...`, Cargo starts the
compiler and passes `build` and its arguments to it.

## 2. Arca Project Anatomy

Arca is Actus's project and package workflow. A project has a manifest that
describes its package identity and build contract, and a source root containing
Actus modules.

Create a project in a new directory:

```text
$ actus new hello --no-git
created `hello`
$ cd hello
```

`new` takes the destination name. By default, project creation initializes
Git unless the project is already inside a Git repository. `--no-git`
explicitly disables that step. The equivalent configuration form is
`--vcs none`.

For an existing directory, use:

```sh
actus init --no-git
```

The generated project has this shape:

```text
hello/
├── Arca.toml
├── .gitignore
└── src/
    └── main.act
```

The generated `Arca.toml` is:

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "alpha"
entry = "main"

[build]
target = "host"
profile = "debug"
```

The `[package]` table identifies the package. `name` is the package
namespace, `version` is its package version, and `edition` selects the
language edition. `entry = "main"` tells the hosted build which Actus
`verb` is the executable entry point. The `[build]` table selects the host
target and the default debug profile.

The manifest is not source code. It does not define ownership or type rules.
It gives the build system the information needed to select a source root,
target, profile, linker contract, and package graph. Dependency and lockfile
behavior is covered in Chapter 09.

The generated `.gitignore` contains:

```text
/capsula/
*.o
*.bin
```

`capsula/` is reserved for generated build metadata and artifacts. Object
files and binary outputs are local products of compilation, not source files.

The `src/main.act` file is the initial source module:

```actus
verb main() -> Int {
    return 0;
}
```

The default source root is `src/`. When the compiler receives an input file,
it searches upward for `Arca.toml`, reads `package.source_root` when one is
configured, and otherwise uses `src/`. Directory modules and their facade
rules are described in Chapter 08.

## 3. The First Action

The first program contains one action:

```actus
verb main() -> Int {
    return 0;
}
```

Read it from left to right.

`verb` is a declaration keyword. It says that the following name introduces
an executable Actus action. Actus uses `verb` to make the operation itself
the center of the declaration.

`main` is the identifier of the action. In a hosted executable, the
manifest's `entry = "main"` selects this verb. The empty parentheses `()`
are the parameter list and mean that the action receives no arguments.

The arrow `->` introduces the return type. `Int` is the explicit integer
return type. The return type is part of the compiler-checked contract; it is
not inferred from the `return` statement for this entry boundary.

The opening brace `{` starts the action body and the closing brace `}`
ends it. Inside the body, `return` terminates the action and supplies its
result. The literal `0` is an integer value, and `;` terminates the
statement. For a hosted executable, the configured entry contract maps the
returned integer to operating-system process status `0`.

```text
Actus main() returns 0
          │
          ▼
native entry boundary
          │
          ▼
operating system records exit status 0
```

The operating system does not receive an Actus object or a pointer to the
source variable. It receives the target ABI's integer return value after the
native entry code has completed.

### Role-qualified calls in a project

The same source file can make a temporary mutation explicit at the call site:

```actus
verb append_marker(ins buffer: Buffer) -> Int {
    append(buffer, 41);
    return 0;
}

verb main() -> Int {
    erg buffer = Buffer[4];
    append_marker(buffer: ins buffer);
    append(buffer, 1);
    return 0;
}
```

`Buffer[4]` creates the owned buffer binding. The `ins` marker after the
argument label is required because the callee requests an exclusive call-scope
loan. During `append_marker`, the caller cannot read, move, drop, or create
another borrow of `buffer`. The call returns the loan and restores the same
owner, so the following `append` is checked against the restored mutable state.
No heap wrapper or lifetime annotation is added to the call.

## 4. Compilation and Execution

There are two useful workflows. `build` leaves a selected artifact on disk;
`run` builds a temporary executable, runs it, returns its status, and removes
the temporary artifact. When invoked from a project directory, both commands
discover `Arca.toml` and use its configured entry source. An explicit `.act`
path remains available when a different source is the intended input.

Build an executable explicitly:

```text
$ actus build src/main.act --emit exe
built `capsula/debug/<target-triple>/main.bin`
$ ./capsula/debug/<target-triple>/main.bin
$ echo $?
0
```

The input to `build` is `src/main.act`. `--emit exe` requests a linked
executable rather than the default object emission. Without `-o`, Actus writes
the artifact beneath `capsula/<profile>/<target-triple>/`; the exact triple is
platform-dependent. The compiler prints the successful build message to
standard output. The program itself prints nothing, so the terminal is empty
between the launch and `echo $?`. The shell expands `$?` to the previous
process's exit status.

The same program can be compiled and run without retaining the executable:

```text
$ actus run
42
process exited with status 0
$ echo $?
0
```

`run` does not expose the temporary executable path. Program output is kept on
stdout, while the process status and compiler diagnostics are sent to stderr.
The status returned by `actus run` is the Actus program's status, so shell
scripts can use it directly.

Build profiles are selected at the command line:

```sh
actus build --release --emit exe
actus run --profile debug
```

The debug profile uses the configured non-optimizing Cranelift level. The
release profile selects the speed-oriented level. Profile selection changes
backend settings; it does not weaken semantic validation or ownership checks.

Before generating native code, use `check` for a fast frontend-only
validation:

```text
$ actus check
checked `src/main.act` successfully
```

`check` reads the source, resolves modules, parses it, and runs semantic
analysis without emitting an object file or executable.

For a package that selects another source root or entry in `Arca.toml`, the
same commands follow that manifest contract automatically:

```toml
[package]
source_root = "app"
entry = "start"
```

From the package root, `actus check`, `actus build`, and `actus run` resolve
`app/main.act` as the source entry and use the configured entry contract. A
profile affects native code generation only; checking, formatting, and test
discovery remain semantic operations.

Use `actus watch` when the command should remain active and re-check after a
source or manifest change. It is intentionally separate from one-shot `run`:

```text
$ actus watch
watching `src/main.act`
checked `src/main.act` successfully
```

`actus watch --once` is a deterministic CI-friendly form. `--build` performs
an executable build after each detected change, and Ctrl-C ends the watcher
with status 130.

## 5. What Arca and the Compiler Actually Do

When `actus build` starts, it first identifies the input file and discovers
the project configuration. The configuration search walks from the input
directory toward its ancestors until it finds `Arca.toml`. The manifest
selects the package source root, target specification, entry contract, and
profile.

The compiler then reads the source as bytes. The lexer converts characters
into tokens and reports invalid characters or unterminated literals. The
parser converts tokens into an AST containing declarations, statements,
expressions, types, roles, and source spans. If the project imports modules,
the Directory Module Resolver finds the canonical facade and its direct
siblings, orders them deterministically, and applies the facade's public
export boundary.

Semantic analysis receives the resulting program. It registers types and
verbs, resolves names and imports, checks parameter and return types, and
validates `erg`, `abs`, `dat`, and `ins` transitions. It rejects
use-after-move, invalid borrows, aliased exclusive loans, unknown types,
incompatible returns, and invalid entry contracts before the backend is
invoked. For an `ins` call, the caller's owner becomes `Suspended` only for
the call and returns to `Active + Mutable` when the call completes.

An access-qualified return is checked as part of the same contract. A verb
declared with `-> abs Type` returns a non-owning view, not a new owner. The
semantic analyzer requires one unambiguous `abs` origin and records the view in
the caller's scope so the source remains frozen until the view ends.

The validated AST and semantic model are passed to the reference Rust
bootstrap compiler's Cranelift backend. Cranelift lowers the program into
target-aware SSA-style IR, including function calls, integer operations,
layouts, ownership cleanup actions, and the selected entry symbol. The
backend emits an object file. The system linker combines that object with
startup code and required libraries.

The final file format is target-dependent:

```text
Actus source
    │
    ▼
Arca.toml + source discovery
    │
    ▼
lexer → parser → AST
    │
    ▼
semantic analysis and cleanup planning
    │
    ▼
Rust bootstrap compiler → Cranelift IR
    │
    ▼
native object file
    │
    ▼
system linker
    │
    ├── ELF    (typical Linux target)
    ├── Mach-O (typical macOS target)
    └── PE     (typical Windows target)
```

The target specification supplies architecture, pointer width, endianness,
object format, ABI, linker flavor, and entry contract. The backend does not
blindly use the host machine's settings when a project target says otherwise.
The build graph records a target specification hash so an artifact built for
one target contract is not silently reused for another.

## 6. Diagnostics and Compiler Guardrails

A compiler error is a statement about a failed proof. Consider a return type
mismatch:

```actus
verb main() -> Int {
    return true;
}
```

The declaration promises an `Int`, but `true` is a `Bool`. A typical
diagnostic has this shape:

```text
error[E1026] at 2:12: return type mismatch: expected Int, found Bool
```

The stable code is the first thing to record when searching documentation or
reporting a compiler issue. The location is one-based line and column
information. Here, line 2 contains the invalid return expression. The message
names both the expected and actual types, so the repair is local: either
return an integer or change the action's declared return type if that is the
intended contract.

A syntax error is reported before semantic analysis. For example, removing
the closing brace leaves the parser without the token required to finish the
body:

```text
error[E0004] at 3:1: expected `}`, found end of input
```

The compiler does not continue to code generation after a lexical, parse, or
semantic failure. This ordering prevents the backend from guessing what the
program meant. Diagnostics are rendered from structured error values, so the
same frontend facts can later be used by the LSP server and editor clients.

## 7. Summary

At the end of this chapter, the workflow is:

```text
create project → edit the configured entry → check → build → run or watch
```

`Arca.toml` describes the project and build contract, including source root,
entry, target, and profile settings. The configured entry source contains the
program. `actus check` validates without code generation. `actus build`
produces an object or linked executable. `actus run` executes a temporary
linked executable and returns its process status. `actus watch` repeats
checking or building after relevant source changes.

In Chapter 03, we will examine the Actus lexicon and syntax in detail:
keywords, identifiers, literals, declarations, parameters, types, punctuation,
and the rules that turn source text into the tokens consumed by the parser.
