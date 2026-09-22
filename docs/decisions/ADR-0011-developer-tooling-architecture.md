# ADR-0011: Developer Tooling Architecture

- Status: Accepted future design
- Date: 2026-09-22
- Scope: Actus formatter, editor syntax engine, and language server

## Context

Actus needs consistent source formatting, responsive editor support, and
semantic language intelligence. These capabilities must remain separate from
one another and must not duplicate compiler ownership, borrowing, or
diagnostic logic.

## Decision

Developer tooling is divided into three components:

```text
actus fmt
tree-sitter-actus
actus lsp
```

Their responsibilities are deliberately distinct.

## `actus fmt`: Canonical Formatter

`actus fmt` is the single authoritative formatter for Actus source code. It
uses one deterministic style with no user configuration.

Formatter output must be idempotent:

```text
fmt(fmt(source)) == fmt(source)
```

Formatting must preserve program semantics and must retain source trivia such
as comments, whitespace where meaningful, empty lines, punctuation, and source
spans. The formatter therefore consumes a lossless, trivia-aware syntax
representation rather than relying on a semantic AST alone:

```text
source -> trivia-aware lexer -> lossless CST -> AST -> semantic model
```

The formatter does not perform ownership checking, semantic analysis, or code
generation. Malformed input produces structured diagnostics instead of
partially emitting misleading output.

## `tree-sitter-actus`: Editor Syntax Engine

`tree-sitter-actus` is maintained as a separate grammar repository. Its
initial structure is:

```text
tree-sitter-actus/
├── grammar.js
├── queries/
│   ├── highlights.scm
│   ├── folds.scm
│   └── indents.scm
└── test/
```

It provides only editor-facing incremental syntax services:

- syntax highlighting;
- folding;
- indentation hints;
- incremental parsing;
- error-tolerant syntax trees.

Tree-sitter must not implement ownership analysis, borrow checking, type
checking, semantic diagnostics, or code generation. Its grammar follows the
Actus language specification but is not the compiler's semantic source of
truth.

## `actus lsp`: Semantic Language Server

`actus lsp` is a compiler CLI subcommand and exposes the Language Server
Protocol, normally over standard input and output:

```sh
actus lsp
```

It is built on the compiler frontend:

```text
document
  -> lexer
  -> error-resilient parser
  -> AST/CST
  -> semantic analyzer
  -> LSP responses
```

The language server provides:

- syntax and semantic diagnostics;
- ownership and borrow diagnostics;
- semantic highlighting for `erg`, `abs`, and `dat`;
- hover information;
- document symbols and navigation;
- completion and references;
- document and range formatting requests.

LSP parsing must tolerate incomplete editor documents without weakening the
strict batch compiler. Recovery parsing may represent missing delimiters,
partial declarations, and temporary syntax errors while preserving as much
semantic context as possible.

## Semantic Highlighting

Keyword coloring is insufficient for Actus roles. The language server obtains
semantic information from the analyzer, including owner, borrow, moved,
frozen, task, and dropped states where available.

Tree-sitter may provide lexical highlighting, but it must not claim semantic
ownership or borrow state.

## Formatter Integration

The language server must delegate formatting to the canonical formatter:

```text
LSP formatting request -> Actus formatter -> LSP TextEdit response
```

The LSP must not contain a second formatting implementation. CLI and editor
formatting must produce identical canonical output.

## Architectural Boundaries

```text
CST preserves text.
AST represents meaning.
Semantic analyzer explains meaning.
Formatter canonicalizes syntax.
Tree-sitter serves editors.
LSP exposes compiler intelligence.
```

The following are prohibited:

- duplicating semantic analysis inside the LSP;
- implementing borrow checking in tree-sitter;
- creating an independent grammar inside the formatter;
- using tree-sitter as the native code-generation frontend;
- allowing editor indentation rules to redefine compiler semantics.

## Future Source Structure

The compiler may eventually add:

```text
src/
├── syntax/
│   ├── trivia.rs
│   ├── cst.rs
│   └── recovery.rs
├── formatter/
└── lsp/
```

This structure is a future decomposition guide. It does not require an
immediate split of the current Rust bootstrap package.

## Consequences

Actus receives one canonical formatter, a lightweight incremental editor
syntax engine, and a semantic language server backed by the real compiler
frontend. Clear boundaries prevent tooling drift and keep language semantics
centralized in the compiler.

