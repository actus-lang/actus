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

The attribute applies to a verb declaration. It is normally placed in a
mirrored file under `tests/`, where the test receives the corresponding
module's internal test scope without making private declarations public to
ordinary importers.

Package-root test files remain supported as compatibility fixtures. New tests
should mirror the `src/` tree and can be checked with:

```sh
actus test --mirror --strict
```

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
actus test --file ie/runtime/training/auxiliary/cursor.act --strict
actus test --list
```

Use `--exact` when a filter must match exactly.

## Constraints

- The attribute is valid only on supported test declarations.
- A test fixture must remain deterministic and focused.
- Fault injection is separate metadata and is allowed only under the test
  runner's controlled fault-injection configuration; it is not an application
  API.

