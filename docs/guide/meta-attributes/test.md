# `meta test`

`meta test` marks a verb as a test entry for `actus test`. The runner discovers
the marked verb, applies the requested module/file filters, compiles the test
artifact, and reports its process result.

```actus
meta test
verb accepts_empty_input() -> Int {
    return 0;
}
```

## Scope and placement

A `meta test` verb applies to a verb declaration and lives in one of two
places (ADR-0121):

- **Module tests** live in the `tests` directory next to the module facade.
  `src/ie/persistence/tests/*.act` tests the module `ie::persistence`
  (`src/ie/persistence/persistence.act`). The files are compiled as part of the
  module, so a module test sees the module's private declarations and the
  `open` declarations of its submodules and imports. It does not see a
  submodule's private declarations (`E1119`); the submodule's own `tests`
  directory tests those. The root unit has no `tests` directory: move its logic
  into a module, or test it from `tests/` through what it opens.
- **Project tests** live in `tests/*.act` at the project root. They test the
  program from outside and see only what any importer sees: the `open`
  declarations that facades export. Calling a private declaration is `E1109`.

All files of one `tests` directory share one namespace, so a helper verb,
struct or constant declared in one file is usable from the others. Declaring the
same name in two files is `E1106` and names both files. A `tests` directory
holds `.act` files only (a subdirectory named `fixtures` or `snapshots` is
ignored as test data), is never a module (a `tests/tests.act` facade is
`E1143`), and `actus build`, `actus run` and `actus check` ignore it. `actus fmt`
formats it.

A `meta test` verb in any other source file is `E1144`, and an `.act` file in a
subdirectory of `tests/` (the old mirror layout) is `E1143`; both messages name
the module `tests` directory the test belongs in. The `--mirror` option no
longer exists.

A test is named `<module path>::<verb>` for a module test
(`ie::persistence::accepts_header`) and `tests::<verb>` for a project test.

## Runtime and safety contract

`meta test` changes test discovery, not the semantics of the verb in a normal
build. It does not export private symbols, weaken ownership, or add a runtime
test framework to the program.

The test runner may compile a temporary test entry and execute it as a child
process. A non-zero result, timeout, signal, or failed compilation is a test
failure.

## Filters

Tests can be selected by name, module, or source file:

```sh
actus test persistence --strict
actus test --module ie::runtime --strict
actus test --file ie/runtime/tests/cursor.act --strict
actus test --list
```

Use `--exact` when a filter must match exactly.

## Constraints

- The attribute is valid only on supported test declarations.
- A test fixture must remain deterministic and focused.
- Fault injection is separate metadata and is allowed only under the test
  runner's controlled fault-injection configuration; it is not an application
  API.

