# Alpha Application Workflow

Gate 19.2 is the executable standard-library application baseline. The
examples are real Actus entry points; `tests/applications.rs` copies the
standard-library sources into an isolated project, then checks, builds, and
runs each entry point through the native backend.

## Console application

`examples/console_application.act` reads one line through `std::io`. A
successful input is written independently to stdout and stderr and the
program exits with status `0`. Empty input produces `IoError.EndOfStream`,
writes a short diagnostic to stderr, and exits with status `2`.

## File utility

`examples/file_utility.act` constructs a raw POSIX path, normalizes it,
inspects its relative and file-name components, writes `ACT`, reads it back,
checks metadata, removes the file, and then verifies the typed missing-file
failure. The application owns all returned buffers and relies on standard
library cleanup for file handles.

## Reproduce the evidence

From the repository root, run the application evidence test:

```sh
cargo test --test applications -- --nocapture
```

The test asserts native compilation, exact output bytes, independent process
statuses, typed EOF and missing-file failures, and removal of the temporary
file. The fixture source is kept under `tests/fixtures/applications/`; no
test logic is embedded in the standard-library implementation.
