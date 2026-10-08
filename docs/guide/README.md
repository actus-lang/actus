# Actus Handbook

This is the user handbook for the Actus programming language, compiler, and
standard library.

The handbook explains how to read and write Actus code, how ownership and
module boundaries work, how to use the standard library, and how to check,
test, format, and build a project.

## Choose a reading path

### First Actus program

1. [Language overview](language/overview.md)
2. [First program](language/first-program.md)
3. [Source files and project layout](language/source-files-and-layout.md)
4. [Types and literals](language/types-and-literals.md)
5. [Verbs and contracts](language/verbs-and-contracts.md)
6. [Expressions and statements](language/expressions-and-statements.md)
7. [Project setup](workflow/project-setup.md)
8. [Check, test, and build](workflow/check-test-and-build.md)

### Working with ownership

1. [Ownership overview](ownership/overview.md)
2. [Roles](ownership/roles.md)
3. [`erg`](ownership/erg.md)
4. [`abs`](ownership/abs.md)
5. [`dat`](ownership/dat.md)
6. [`ins`](ownership/ins.md)
7. [Moves and scalar reuse](ownership/moves-and-reuse.md)
8. [Cleanup and scope](ownership/cleanup-and-scope.md)
9. [Ownership diagnostics](ownership/common-diagnostics.md)

### Intermediate Actus work

1. [Ownership overview](ownership/overview.md)
2. [Structs and enums](language/structs.md)
3. [Arrays and buffers](language/arrays-and-buffers.md)
4. [Arenas](language/arenas.md)
5. [Modules and facades](modules/overview.md)
6. [Errors and results](library/errors/README.md)
7. [Debugging](workflow/debugging.md)

### Advanced Actus work

1. [Generics](language/generics.md)
2. [Packs](language/packs.md)
3. [Constants](language/constants.md)
4. [Regions](library/region/README.md)
5. [Native builds](compiler/native-builds.md)
6. [Wire](library/wire/README.md)
7. [Benchmarks](workflow/benchmarks.md)

### Building a project

1. [Project setup](workflow/project-setup.md)
2. [`Actus.toml`](compiler/actus-toml.md)
3. [Modules and facades](modules/overview.md)
4. [Runtime profiles](library/runtime-profiles.md)
5. [Targets and profiles](compiler/targets-and-profiles.md)
6. [Check, test, and build](workflow/check-test-and-build.md)
7. [Native builds](compiler/native-builds.md)
8. [Diagnostics](compiler/diagnostics.md)

### Using the standard library

1. [Standard-library overview](library/README.md)
2. [IO](library/io/README.md)
3. [Filesystem](library/filesystem/README.md)
4. [Paths](library/path/README.md)
5. [Strings and UTF-8](library/string/README.md)
6. [Time](library/time/README.md)
7. [Regions](library/region/README.md)
8. [Wire](library/wire/README.md)
9. [Errors and results](library/errors/README.md)

### Coding-agent reference

1. [Agent reading order](agent-reference/reading-order.md)
2. [Source-change checklist](agent-reference/source-change-checklist.md)
3. [Evidence and scope](agent-reference/evidence-and-scope.md)
4. [Repository boundaries](agent-reference/repository-boundaries.md)
5. [Diagnostic decision tree](agent-reference/diagnostic-decision-tree.md)

## Shared terminology

Use [the terminology page](terminology.md) for the words and notation used
throughout the handbook.

## Categories

- [Language](language/README.md)
- [Ownership](ownership/README.md)
- [Modules](modules/README.md)
- [Standard library](library/README.md)
- [Detailed library guides](library/README.md)
- [Compiler](compiler/README.md)
- [Workflow](workflow/README.md)
- [Agent reference](agent-reference/README.md)

## How to read examples

Examples in this handbook use current Actus syntax. When an example is tied
to a package, runtime profile, target, or standard-library module, the text
states the required project setup beside the example.

Actus-specific terms are defined in ordinary language before their detailed
rules are introduced. Links point to the focused page for the complete
contract.

## Source and implementation references

- [Guide migration map](source-map.md)
- [Compiler pipeline](../architecture/compiler-pipeline.md)
- [Language documents](../language/alpha-user-guide.md)
- [Examples](../../examples/)
- [Standard-library source](../../library/std/src/)
- [Tests](../../tests/)

The migration map records where the original coding guide belongs in this
handbook. The handbook pages are the reader-facing entry point; implementation
source and tests remain the detailed references for compiler behavior.
