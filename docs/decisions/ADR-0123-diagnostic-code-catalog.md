# ADR-0123: Diagnostic Code Catalog and `actus explain`

## Status

Accepted.

## Context

Every compiler error carries a stable `E####` code, and a code keeps its
meaning within an Alpha language edition. What a code means is written down
only in part:

- `docs/language/diagnostics.md` lists code ranges with a category each
  (`E1040`–`E1079`: "Enum patterns, roles, generics, packs, and arenas") and
  describes a few codes one by one. The compiler emits about 190 codes.
- No code has an example of a program that triggers it, or of the fix.
- Nothing checks the list against the compiler, so a new code, a moved code or
  a code that is no longer emitted goes unnoticed.

The missing check has already let two meanings share codes. The renderer
formats lexer errors as `E000{n}`, so an unterminated doc string, an invalid
integer type, an invalid hexadecimal literal and an invalid float type report
`E0003` to `E0006`, which are also the parser's unexpected token, unexpected
end of input, duplicate name and unknown metadata. `docs/language/lexical-map.md`
documents the hexadecimal error as `E0005`.

A user who meets a code needs, in one place: what it means, a program that
produces it and how to fix that program. The same text belongs in the
terminal, in the editor and on the website, and it must not drift from the
compiler.

## Decision

### One entry per code

The catalog is a directory, `docs/language/diagnostics/`, with one Markdown
file per code, named after it: `docs/language/diagnostics/E1140.md`. An entry
has a fixed shape:

~~~~markdown
# E1140: operands of different integer types

One paragraph or more: what the rule is, why it exists and which ADR set it.

## Example

```actus fails=E1140
verb main() {
    erg small: u8 = 200u8;
    erg large: u32 = 100000u32;
    erg sum = small + large;
}
```

## Fix

Explanation of the fix.

```actus compiles
verb main() {
    erg small: u8 = 200u8;
    erg large: u32 = 100000u32;
    erg sum = (small as u32) + large;
}
```
~~~~

- The title line is `# E####: <summary>`, with a summary of at most 60
  characters in lower case.
- `## Example` holds at least one block marked `fails=E####`, with the entry's
  own code. `## Fix` holds at least one block marked `compiles`. An entry may
  show several examples and fixes.
- An example that needs more than one file is a group of blocks that each name
  their file: `actus fails=E1109 file=src/engine/engine.act`,
  `toml file=Actus.toml`. A group is built as one project. A block without
  `file=` is `src/main.act` of a project with the default manifest.
  The blocks of one section form one project, in order; a block whose file
  is already in the project starts the next one, so a section can show
  several single-file examples.
- An error that no program can produce on purpose, such as a source file that
  cannot be read, has no `## Example`. It has `## Cause` instead, which says
  what triggers it, and `## Fix` without a `compiles` block.

The catalog lives in the compiler repository because it describes the
compiler; `docs/language/diagnostics.md` keeps the ranges and points to it.

### The catalog is checked

A test in the compiler's own suite fails when the catalog and the compiler
disagree:

1. Every code that the compiler can emit has an entry, and every entry names a
   code that the compiler can emit. The set of emitted codes is read from one
   registry in the compiler (below), not from source text.
2. Every `fails=E####` example, built with `actus check`, reports exactly that
   code as its first error, at a location inside the example.
3. Every `compiles` block passes `actus check`.
4. Every entry has the shape above.

Until every code has an entry, the test reads a list of the codes that do not
have one yet. A code on the list is skipped by check 1 and only by it; a
listed code that has an entry, or a code that is neither listed nor entered,
fails the test, so the list can only shrink and a new code cannot join it.
The list is deleted when it is empty.

The examples run on both back ends when the code is a back-end or build error
rather than a check error; the marker `fails=E####,build` says the example
fails in `actus build` instead of `actus check`.

An example whose result depends on the target, such as a capability, a
stack-limit proof or a platform selector, names its target in a
`toml file=Actus.toml` block, so the catalog gives the same result on every
machine that runs it.

The marker `fails=E####,test` says the example fails in `actus test`, which
is the only command that reads a `tests` directory (ADR-0121). A `compiles`
group that holds a file under a `tests` directory is verified with `actus test`
and must exit 0, so every test of it passes.
The marker `fails=E####,strict`, also written `fails=E####,build,strict` or
`fails=E####,test,strict`, runs the command with `--strict` (ADR-0041), and a
`compiles` group written `actus compiles strict` is verified with `--strict`.
The marker `llvm`, written `fails=E####,llvm` (last, after `build`/`test` and
`strict`) or `actus compiles llvm`, says the group needs a compiler built with
the `llvm` feature, as a `meta stack_limit` proof does (ADR-0103). A test binary
built without that feature skips the group and prints how many groups it
skipped; every other group runs on every build. A group that needs the proof
also pins its target in `Actus.toml`, so the result does not depend on the host.
Every diagnostic prints as `<path>: error[E####] at line:column: message`, or
`error[E####]: message` when it names no file, which only an error of the
toolchain or the environment does; the test skips the location check of such a
line only for E1803, E1806, E1880 and E1881, and any other code must name a file
inside the example.
A comment line that holds `catalog: repeat N` starts a region of a block and a
comment line that holds `catalog: end` ends it, using the line comment of the
block's language (`#` in Actus and TOML). When the test builds the project it
writes the lines of the region N times and replaces `{n}` in them by the
1-based index, zero-padded to the width of N, so an example that needs hundreds
of lines of filler shows the pattern once. The marker lines are written as they
are; regions do not nest, and an unclosed region, a count of zero or a count
that is not a number is an error of the entry.

### One registry of codes

The codes move into one place in the compiler: a `DiagnosticCode` type with one
constant per code, and every diagnostic is built from it instead of a string
literal or a formatted number. Two meanings can then no longer share a code
without the registry showing both names for one value, and a test checks that
no two constants have the same value. The test of the catalog takes the
emitted codes from this registry.

### Codes that change

The four lexer errors that share a code with the parser get codes of their own,
next after the last parser code:

| Error | Was | Becomes |
| --- | --- | --- |
| Unterminated doc string | `E0003` | `E0024` |
| Invalid integer type | `E0004` | `E0025` |
| Invalid hexadecimal literal | `E0005` | `E0026` |
| Invalid float type | `E0006` | `E0027` |

The move of a field out of a `zero_on_drop` owner (ADR-0102) shares `E1136`
with the dropped `deterministic` contract of a role method (ADR-0100). It gets
a code of its own, `E1145`; `E1136` keeps the meaning that ADR-0100, the guide
and the tests give it.

`E0001` and `E0002` keep their meaning, and `E0003` to `E0006` keep the
parser's. The stability rule protects a code's meaning; these five codes never
had one meaning, so the change restores the rule instead of breaking it.

### Retired codes

A code the compiler can no longer emit is retired: its constant leaves the
registry, its number stays in the registry's `RETIRED` list and is never given
to another diagnostic, and it has no catalog entry. A test checks that no
active code is retired and that no retired code has an entry or is listed in
`missing.txt`. `docs/language/diagnostics.md` names each retired code, with the
code that now covers its case.

### `actus explain`

`actus explain E1140` prints the entry for a code: the title, the text, the
examples and the fixes, with the Markdown markers of headings and fences
removed and code shown as plain indented text. The catalog is embedded in the
compiler binary at build time, so the command works offline and matches the
compiler it ships with.

- A code without an entry, or an argument that is not of the form `E####`, is
  an error with exit status 2 that names the argument.
- `actus explain` without an argument is the same usage error as any command
  that misses its argument.

When `actus check`, `actus build`, `actus run` or `actus test` reports errors,
the last line of the report names the command once:

```text
For more information about an error, run `actus explain E1140`.
```

The line names the first code of the report. It is not printed for the
machine-readable diagnostics of the language server.

### The website

The website builds one page per code and an index grouped by the existing
ranges from the catalog of the release it documents. It does not hold its own
copy of the text.

## Consequences

- A code has one meaning, one explanation and examples that are checked
  against the compiler on every test run.
- A change that adds a code must add its entry, and a change that alters a
  rule must keep its examples true, in the same commit.
- Programs or scripts that match the five renumbered codes must match the
  new ones; the tests and documents that name them change with this
  decision.
- Every test that compares the full stderr of a failing command gains the
  `actus explain` line.
- Writing about 190 entries is the bulk of the work; the registry, the test
  and the command are small. The roadmap orders the work so the check exists
  before the entries, and each range of entries lands with its examples
  verified.

## Alternatives considered

### Keep the catalog only on the website

Rejected. Nothing could check it against the compiler, `actus explain` would
need the network, and the text would drift with every release.

### Put the explanation in the Rust source next to each error

Rejected. The examples are Actus programs with Markdown around them; long
string literals in the semantic code would mix documentation with the rule
that emits it, and the Markdown files can be read and reviewed on their own.

### Collect the emitted codes by searching the source for `E####`

Rejected. Codes are produced by literals and by formatted numbers, so a
search misses some and finds others in comments; the lexer collision is an
example. A registry makes the set of codes a value the test can read.
