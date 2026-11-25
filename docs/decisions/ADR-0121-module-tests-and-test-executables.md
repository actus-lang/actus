# ADR-0121: Module Tests and One Test Executable per Group

## Status

Accepted.

## Context

`actus test` builds one native executable for every `meta test` verb. Each
build runs the whole pipeline for the test's program: frontend, code
generation, object emission and linking. A project whose tests import a large
module pays that cost once per test. A suite of about two hundred tests that
import the same engine takes about an hour, almost all of it spent building
the same code again.

Where a test lives is also loose. A test can be a `meta test` verb in a
production source file, a file under `tests/` that mirrors a source path and
is compiled into that module (`--mirror` checks the mirror), or a file
directly under `tests/`. All three see private declarations: the first two of
the module they belong to, the third of every module it imports. Nothing in
the layout says which kind of test a file is, private tests sit far from the
code they test, and no test checks a module only through its open interface.

ADR-0099 kept every test in its own executable and allowed grouping only
"when every selected test ... does not rely on process-level isolation". That
condition is the reason grouping has not happened: a runtime check stops the
whole process (ADR-0117), so tests that share one process cannot isolate a
failure.

## Decision

### Two places for tests

1. **Module tests** live in a `tests` directory inside the module's directory,
   next to its facade: `src/ie/persistence/tests/*.act` tests the module
   `ie::persistence` (`src/ie/persistence/persistence.act`).
2. **Project tests** live in `tests/` at the project root. They test the
   program from outside.

The root unit has no test directory: `src/tests/` is an error. Code in the
root unit that needs tests of its own belongs in a module; the root unit is
otherwise tested from `tests/` through what it opens. A `tests` directory
holds `.act` files only, with no subdirectories (`E1143`).

`meta test` verbs are allowed only in these two places. A `meta test` verb in
any other source file is an error that names the module's `tests` directory.
A file under `tests/` in a subdirectory, which used to mirror a source path, is
an error that names the module `tests` directory it belongs in. The `--mirror`
option is removed.

A module `tests` directory is not a module. A directory named `tests` with its
own facade (`tests/tests.act`) inside the source root is an error, so the name
cannot mean both.

### What a test sees

A test sees exactly what code in its place sees.

- A module test file is compiled as part of its module. It sees the module's
  private declarations, and the open declarations of the module's submodules
  and imports, through the ordinary visibility rules (ADR-0047, ADR-0110). It
  does not see a submodule's private declarations; the submodule's own
  `tests` directory tests those.
- A project test sees only what any importer sees: the open declarations that
  facades export. This is a change: a project test used to see the private
  declarations of every module it imported.

Test files never change what production code sees. `actus build`, `actus run`
and `actus check` ignore every `tests` directory; `actus fmt` formats them.

### One namespace per test directory

All files of one test directory share one namespace, as all files of a Go
package do. For a module, that is the module's namespace. For `tests/`, it is
one namespace for the project tests. Two declarations with the same name in
one test directory are an error that names both files. Helper verbs, structs
and constants can therefore be shared between the files of a directory.

An import of the same module in several files of `tests/` is one import, `open`
when any of them is. A module test file's imports are checked as those of any
sibling file of the module. A subdirectory named `fixtures` or `snapshots` is
test data, not tests, and is ignored.

### One executable per test directory

The runner builds one executable per test directory that has a selected test:
one for each module `tests` directory and one for `tests/`. The executable
contains every test of its directory and a generated entry that runs one test,
chosen when the process starts.

Each test still runs in its own process. The runner starts the directory's
executable once per selected test and tells it which test to run. A test that
fails, traps, times out or is killed by a signal ends only its own process.
Working directory, timeout, fault injection, the exit-status contract, output
capture and reporting stay as they are per test (ADR-0099 sections 1, 3 and 5).
This meets the isolation condition of ADR-0099 section 4 while building each
directory once.

Executables are built in parallel, bounded by `--test-threads`; tests run in
parallel under the same bound. Output order stays the canonical test order.

### Names and filters

A test is named by its directory and verb: `ie::persistence::accepts_header`
for a module test and `tests::accepts_header` for a project test. Names are
unique because a directory has one namespace. `FILTER`, `--module`, `--file`,
`--exact` and `--list` keep their meaning; `--module ie::persistence` selects
that module's tests, and `--file` matches the test file's path.

## Consequences

- A suite builds once per test directory instead of once per test.
- Private tests sit next to the module they test, and a project test can only
  use the public interface.
- Existing tests move: a mirrored file `tests/<path>.act` goes to the module's
  `tests` directory, an inline `meta test` verb goes to a file there, and a
  project test that calls a private declaration becomes a module test of that
  declaration's module. Helpers that were duplicated between files of one
  directory are merged.
- A test name no longer contains its file name: `tests::order::alpha` becomes
  `tests::alpha`.
- The `--mirror` option and the mirror rules of the test guide are removed.
- ADR-0099 section 4 is superseded: grouping is the default, with isolation
  kept by running each test in its own process.

## Alternatives considered

### Run every test of a group in one process

Rejected. A runtime check, an exit or a signal in one test would end the
others, and global state would leak between tests.

### One executable per test file

Rejected. It still builds a module once per file, and files of one module
usually share helpers.

### Let a module test see its submodules' private declarations

Rejected. A change inside a submodule would break its parent's tests, and the
submodule's encapsulation would hold for its importers but not for its parent.
