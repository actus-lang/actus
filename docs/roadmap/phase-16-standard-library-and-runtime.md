# Phase 16: Standard Library, Runtime Boundary, and Tooling Stabilization

This phase makes Actus useful for real programs before self-hosting begins. It
stabilizes the implemented language surface, keeps editor tooling in lockstep,
and establishes a small, explicit runtime boundary for the first standard
library modules.

Self-hosting is intentionally deferred. The phase is complete when Actus can
compile and run practical host-side utilities using its own library interfaces,
without requiring new language syntax or compiler-specific application logic.

## Goals

- Stabilize the current language surface before adding new syntax.
- Synchronize Tree-sitter, VS Code, and LSP behavior with the compiler.
- Keep the `core -> runtime -> std` dependency direction explicit.
- Expose host services only through deliberate `unsafe extern "C"` boundaries.
- Implement the first practical standard library modules:
  `std::io`, `std::fs`, and `std::path`.
- Verify the complete workflow with real Actus programs.
- Preserve a clear, documented boundary for future self-hosting.

## Architectural Invariants

- [ ] Existing Actus syntax remains the source of truth for all tooling.
- [ ] No new ownership role or general control-flow construct is introduced in
      this phase.
- [ ] `erg`, `abs`, `dat`, and `ins` retain their existing contracts.
- [ ] `core` does not depend on an operating system, filesystem, or heap
      service.
- [ ] `runtime` contains only the minimal platform bridge required by `std`.
- [ ] `std` is implemented as Actus-facing library code over explicit runtime
      contracts.
- [ ] Unsafe host access crosses a visible `unsafe extern "C"` boundary.
- [ ] No standard-library API hides ownership transfer or allocation behavior.
- [ ] Host-side behavior has deterministic diagnostics and test results.
- [ ] Bare-metal and `no_std` consumers can exclude host-only modules.
- [ ] Self-hosting remains a future phase and is not a completion criterion.

## Gate 0: Language Surface and Tooling Stabilization

Freeze the currently implemented syntax and bring every developer-facing tool
to the same language contract.

### Compiler surface audit

- [x] Inventory lexer tokens and reserved keywords from the compiler.
- [x] Inventory parser productions for declarations, roles, calls, returns,
      modules, patterns, and expressions.
- [x] Verify `erg`, `abs`, `dat`, and `ins` in declarations and call sites.
- [x] Verify `-> abs Type` in every supported declaration form.
- [x] Verify `Buffer[...]` as the canonical buffer construction syntax.
- [x] Verify `case abs`, `case dat`, and pattern guards.
- [x] Verify `import module;` and `open sibling;` behavior.
- [x] Verify `unsafe extern "C"` declaration constraints.
- [x] Record that the compiler surface and published documentation agree.
- [x] Reject undocumented syntax additions during this gate.

### Tree-sitter synchronization

- [x] Update the grammar for `ins` parameters and explicit `ins` arguments.
- [x] Update the grammar for `abs` access-qualified return types.
- [x] Update grammar rules for modules, imports, and facade exports.
- [x] Update grammar rules for pattern guards and ownership patterns.
- [x] Update `highlights.scm` for all four ownership roles.
- [x] Update `locals.scm` for role parameters, bindings, and pattern names.
- [x] Update `folds.scm` for every supported declaration and case body.
- [x] Update `indents.scm` for blocks, declarations, and patterns.
- [x] Add corpus cases for accepted syntax.
- [x] Add corpus cases for rejected or incomplete syntax.
- [x] Run Tree-sitter generation and corpus tests.

### VS Code synchronization

- [x] Update TextMate grammar scopes for `ins`, `abs`, `dat`, and `erg`.
- [x] Update scopes for `Buffer[...]`, `case`, `open`, and `import`.
- [x] Update scopes for `unsafe extern "C"` declarations.
- [x] Verify `.act` language registration and file associations.
- [x] Verify dark-theme and light-theme readability.
- [x] Rebuild and package the extension.
- [x] Verify installation from the generated `.vsix`.

### LSP synchronization

- [x] Verify diagnostics for the current ownership roles.
- [x] Verify `ins` aliasing and loan-forwarding diagnostics (`E1065`).
- [x] Verify invalid `abs` view-origin diagnostics (`E1066`).
- [x] Verify `Suspended` and `Frozen` access diagnostics (`E1064`, `E1009`,
      and `E1011` where the corresponding semantic state is reached).
- [x] Verify definition lookup through imports and facades.
- [x] Verify hover output for `ins` parameters and `-> abs Type` returns.
- [x] Verify hover output for role-qualified signatures.
- [x] Verify formatting for current module and pattern syntax.
- [x] Verify UTF-16 ranges with multibyte source text.
- [x] Add an integration fixture covering the current complete syntax surface.

### Gate 0 invariant

- [x] A valid Actus source file receives consistent results from the compiler,
      Tree-sitter, LSP, formatter, and VS Code grammar.

## Gate 1: Core, Runtime, and FFI Boundary

Define the smallest platform boundary required by the standard library without
moving operating-system behavior into the compiler core.

### Layer architecture

- [x] Define the public `core` contract and its dependency restrictions.
- [x] Define the `runtime` contract for host services.
- [x] Define the `std` contract over runtime services.
- [x] Document the dependency direction:
      `core -> runtime -> std`.
- [x] Separate host-only services from bare-metal-compatible primitives.
- [x] Define the profile/target behavior for excluding host-only modules.

### FFI boundary

- [x] Define the supported `unsafe extern "C"` ABI surface for runtime calls.
- [x] Define C-compatible scalar and pointer representations.
- [x] Define error/status return conventions at the runtime boundary.
- [x] Define ownership behavior for buffers crossing the boundary.
- [x] Reject hidden allocation at the FFI boundary.
- [x] Reject implicit ownership transfer at the FFI boundary.
- [x] Add positive FFI declaration tests.
- [x] Add negative tests for invalid ABI and ownership combinations.
- [x] Add native linking tests for the runtime bridge.
- [x] Document the host and `no_std` boundary explicitly.

### Gate 1 invariant

- [x] Compiler core remains independent of OS services.
- [x] Every host operation crosses an explicit, typed runtime boundary.
- [x] The same ownership rules apply before and after an FFI call.

## Gate 2: `std::io`

Deliver `std::io` as a sequence of independently verifiable console and
stream capabilities. Each sub-gate must preserve explicit ownership roles,
the hosted-runtime boundary, and deterministic status reporting.

### Gate 2.1: Text & Buffer Console Output

- [x] Define the `std::io` directory facade and public output declarations.
- [x] Define scalar output with `print_int` and `eprint_int`.
- [x] Define borrowed buffer output with `print(abs text: Buffer)`.
- [x] Define newline variants `println` and `eprintln`.
- [x] Define explicit `flush()` for stdout.
- [x] Add length-aware stdout and stderr buffer bridges.
- [x] Add the stdout flush runtime bridge.
- [x] Return an explicit byte-count or failure status from output calls.
- [x] Preserve the source buffer; output never consumes or drops it.
- [x] Keep output bridges behind `unsafe extern "C"` declarations.
- [x] Add semantic facade/export tests for the text API.
- [x] Add native stdout/stderr text execution tests.
- [x] Verify no hidden allocation occurs in the buffer output bridge.

### Gate 2.2: Console Input (Stdin)

- [x] Define `read_line(ins buffer: Buffer)` and its capacity behavior.
- [x] Define `read_byte` and end-of-input behavior.
- [x] Define stdin status and error codes.
- [x] Wrap stdin status codes in `Result[Int, IoError]` at the Actus API boundary.
- [x] Add the runtime stdin bridge behind the approved C ABI.
- [x] Enforce `ins` ownership and call-scope loan restoration.
- [x] Add semantic positive and negative input tests.
- [x] Add native tests for input, end-of-input, and failed reads.

### Gate 2.3: Stream Abstractions (Read & Write)

- [x] Define `read(ins buffer: Buffer)` for stream consumers.
- [x] Define `write(abs buffer: Buffer)` for non-consuming output.
- [x] Define shared I/O status and error-code conventions.
- [x] Keep the C ABI at `>= 0` byte counts, `-1` failure, and `-2` end-of-stream.
- [x] Wrap raw runtime statuses in `Result[Int, IoError]` without offset encoding.
- [x] Define short-read and short-write behavior.
- [x] Map stream operations to target-specific runtime capabilities.
- [x] Add ownership and aliasing tests for read/write calls.
- [x] Add native tests for complete and partial transfers.
- [x] Document public `std::io` ownership, Result, error, and C ABI contracts.

### Gate 2.4: Buffering (`BufReader` & `BufWriter`)

- [ ] Define `BufReader` ownership and refill behavior.
- [ ] Define `BufWriter` ownership, buffering, and flush behavior.
- [ ] Define explicit close/flush failure handling.
- [ ] Ensure internal buffers have deterministic cleanup paths.
- [ ] Add semantic tests for nested views and exclusive refill/write loans.
- [ ] Add native tests comparing buffered and direct stream behavior.

### Gate 2.5: In-Memory Cursor & Stream Utilities

- [ ] Define `Cursor` over an owned or borrowed buffer.
- [ ] Define cursor position, bounds, and seek error behavior.
- [ ] Define stream `copy` ownership and completion semantics.
- [ ] Preserve zero-copy behavior for borrowed cursor views.
- [ ] Add semantic tests for cursor lifetimes and partial copies.
- [ ] Add native tests for cursor reads, writes, and stream copying.

### Gate 2 invariant

- [ ] A complete `std::io` implementation provides deterministic console,
      input, stream, buffering, and in-memory cursor behavior without
      compiler-specific application intrinsics.

## Gate 3: `std::fs`

Provide explicit filesystem operations for host profiles while keeping the
module unavailable or replaceable on targets without filesystem services.

### API contract

- [ ] Define the `std::fs` module facade and public operations.
- [ ] Define path input ownership and borrowing rules.
- [ ] Define file-read result and error semantics.
- [ ] Define file-write result and error semantics.
- [ ] Define create, truncate, append, and overwrite behavior.
- [ ] Define byte-buffer ownership for reads and writes.
- [ ] Define behavior for missing files and permission failures.
- [ ] Define host-profile availability in the target contract.

### Implementation

- [ ] Implement the public `std::fs` declarations in Actus.
- [ ] Implement runtime file-open and file-close bridges.
- [ ] Implement runtime read and write bridges.
- [ ] Implement deterministic error translation.
- [ ] Ensure opened resources have exactly one cleanup path.
- [ ] Add semantic ownership and failure-path tests.
- [ ] Add native tests for read, write, append, and missing-file behavior.
- [ ] Add a target/profile test proving host-only exclusion.

### Gate 3 invariant

- [ ] Filesystem resources are owned, transferred, and cleaned exactly once.
- [ ] No filesystem API is silently available on a target that cannot provide
      its contract.

## Gate 4: `std::path`

Add the path value model required by `std::fs` without coupling Actus source to
one host operating system's path syntax.

- [ ] Define the `std::path` public facade.
- [ ] Define owned and borrowed path representations.
- [ ] Define joining and component inspection operations.
- [ ] Define conversion from supported string/buffer inputs.
- [ ] Define platform separator and encoding behavior.
- [ ] Define invalid-path diagnostics.
- [ ] Add positive path composition tests.
- [ ] Add negative path validation tests.
- [ ] Use the path API from `std::fs` tests.
- [ ] Verify behavior on Linux, macOS, and Windows.

### Gate 4 invariant

- [ ] Path operations are deterministic within a target contract and do not
      leak host-specific assumptions into the language core.

## Gate 5: Real Actus Programs and End-to-End Workflow

Prove that the standard library is usable by application code, not only by
isolated unit tests.

- [ ] Create a console application using `std::io`.
- [ ] Create a file utility using `std::fs` and `std::path`.
- [ ] Compile both projects with `actus build`.
- [ ] Run both projects with `actus run`.
- [ ] Verify stdout and stderr output.
- [ ] Verify process exit codes.
- [ ] Verify `actus check` without code generation.
- [ ] Verify `actus test` against library and application tests.
- [ ] Verify `actus fmt` and `actus fmt --check`.
- [ ] Verify `actus watch --once` and continuous watch behavior.
- [ ] Verify module imports and facade exports from standard-library modules.
- [ ] Verify debug and release profile behavior.
- [ ] Verify deterministic build artifacts and lockfile behavior.
- [ ] Document a complete beginner-to-running-program workflow.

### Gate 5 invariant

- [ ] A real Actus application can use I/O and filesystem services through
      public library APIs without compiler-specific source code.

## Gate 6: Quality, Portability, and Completion

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run `scripts/check_source_limits.sh`.
- [ ] Run Tree-sitter generation and corpus tests.
- [ ] Build and package the VS Code extension.
- [ ] Run Linux CI checks.
- [ ] Run macOS CI checks.
- [ ] Run Windows CI checks.
- [ ] Run coverage and dependency/license policy checks.
- [ ] Verify all public documentation matches the implemented contracts.
- [ ] Verify no hidden runtime, allocator, or ownership behavior was added.
- [ ] Verify all source and function size limits.
- [ ] Record deferred host services and bare-metal substitutions.
- [ ] Record self-hosting as the next major direction, not as part of this
      phase's acceptance criteria.

## Completion Criteria

- [ ] Gate 0 through Gate 6 are complete.
- [ ] `std::io`, `std::fs`, and `std::path` have documented public contracts.
- [ ] Host services are isolated behind the runtime/FFI boundary.
- [ ] At least two real Actus programs use the standard library successfully.
- [ ] All compiler, tooling, portability, and quality checks are green.
- [ ] The resulting workflow is suitable for building the first practical
      Actus utilities before self-hosting work begins.
