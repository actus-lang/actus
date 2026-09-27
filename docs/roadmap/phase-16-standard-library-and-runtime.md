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
- [x] Define explicit `flush()` for stdout with a typed result contract.
- [x] Add length-aware stdout and stderr buffer bridges.
- [x] Add the stdout flush runtime bridge.
- [x] Return `Result[Int, IoError]` with byte counts or typed failure from
      output calls.
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

- [x] Define `BufReader` ownership and refill behavior.
- [x] Verify reusable caller-provided buffers for zero-allocation refill paths.
- [x] Define `BufWriter` ownership, buffering, and flush behavior.
- [x] Define explicit close/flush failure handling.
- [x] Ensure internal buffers have deterministic cleanup paths.
- [x] Add semantic tests for nested views and exclusive refill/write loans.
- [x] Add native tests comparing buffered and direct stream behavior.

### Gate 2.5: In-Memory Cursor & Stream Utilities

- [x] Define `Cursor` over an owned in-memory buffer.
- [x] Define cursor position, bounds, and seek error behavior.
- [x] Define stream `copy` ownership and completion semantics.

#### Gate 2.5.1: Generic Adapter Monomorphization

- [x] Infer concrete generic call arguments from argument and return contexts.
- [x] Synthesize concrete `BufferedReader[Cursor]` and `BufferedWriter[Cursor]` layouts.
- [x] Lower generic adapter methods through direct static performance dispatch.
- [x] Execute native Reader and Writer integration tests with Cursor-backed adapters.
- [x] Add semantic tests for cursor lifetimes and partial copies.
- [x] Add native tests for cursor reads, writes, and stream copying.

#### Future cursor extensions

- [ ] Add a borrowed-view Cursor variant with explicit non-owning lifetime rules.
- [ ] Preserve zero-copy behavior for borrowed cursor views.

### Gate 2 invariant

- [x] A complete `std::io` implementation provides deterministic console,
      input, stream, buffering, and in-memory cursor behavior without
      compiler-specific application intrinsics.

## Gate 3: RAII, Result Propagation, and `std::fs`

This is the current implementation gate. It combines deterministic resource
cleanup, typed error propagation, and the first host filesystem API. The
contracts are defined by ADR-0027 and ADR-0028; filesystem work must use both
contracts rather than introducing parallel cleanup or error mechanisms.

### ADR-0027: RAII scope drop

- [x] Register the deterministic cleanup contract for `File`.
- [x] Emit LIFO cleanup for normal lexical scope exit.
- [x] Emit the same cleanup plan for `return`, `break`, and `continue`.
- [x] Emit cleanup before `?` early return.
- [x] Reject drop while an owner is `Frozen` or `Suspended`.
- [x] Ensure every live owned resource has exactly one drop action.
- [x] Add native exactly-once cleanup tests.

### ADR-0028: Try operator integration

- [x] Parse postfix `?` expressions.
- [x] Resolve `Ok` and `Err` from the expected `Result[T, E]` type.
- [x] Validate compatible error propagation.
- [x] Lower `Ok` unwrapping and `Err` early return natively.
- [x] Preserve cleanup and loan restoration on early return.
- [ ] Extend `?` integration tests to filesystem operations.

### `std::fs` API and implementation

- [x] Define the `std::fs` facade and public file operations.
- [ ] Define `std::path` input ownership and byte-oriented representation.
- [x] Define file-read and file-write `Result` contracts.
- [x] Define create, truncate, append, and overwrite behavior.
- [x] Implement runtime file-open, read, write, and close bridges.
- [x] Translate host failures to typed `IoError` values.
- [x] Ensure opened files use exactly one deterministic cleanup path.
- [ ] Exclude filesystem APIs from targets without filesystem capabilities.
- [x] Add semantic ownership and failure-path tests.
- [x] Add native read, write, append, missing-file, and cleanup tests.

### Production-grade I/O contracts

- [x] Use `Self` receivers for polymorphic `Reader` and `Writer` roles.
- [x] Implement generic buffered adapters without hardcoded stdin/stdout sources.
- [x] Defer `BufferedWriter` target writes until capacity or explicit flush.
- [x] Propagate every `copy` reader and writer error except `EndOfStream`.
- [x] Provide `read_to_bytes` and reject invalid UTF-8 in `read_to_string`.
- [x] Document public types, fields, variants, verbs, and C ABI declarations.

### Gate 3.1: Seeker role and OpenOptions

- [x] Define `SeekFrom` with `Start`, `Current`, and `End` origins.
- [x] Define the `Seeker` role and perform it for `File`.
- [x] Add the runtime seek bridge with bounds and invalid-origin checks.
- [x] Define scalar `OpenOptions` flags and modality helper verbs.
- [x] Add typed `options_open` and the runtime modal-open bridge.
- [x] Add native seek/read-after-seek coverage.
- [x] Add native append coverage proving existing bytes are preserved.

### Gate 3.2: Metadata, filesystem operations, and one-shot I/O

- [x] Define C-layout `Metadata` values and path/open-handle metadata bridges.
- [x] Expose typed path operations for remove, rename, copy, and directories.
- [x] Add one-shot read and write helpers with deterministic `File` cleanup.
- [x] Move filesystem implementation details behind the `std::fs` facade.
- [x] Use caller-allocated storage for struct-returning helper verbs.
- [x] Verify `Result[File, IoError]` and `OpenOptions` helper returns natively.
- [x] Add native metadata size/type assertions.
- [x] Add native rename/remove-file assertions.
- [x] Add native one-shot read/write assertions.

### Gate 3 invariant

- [x] `std::fs` resources are owned, transferred, and cleaned exactly once.
- [x] Every filesystem failure is represented by a typed `Result` value.
- [x] `?` propagates filesystem errors without bypassing cleanup.

### Gate 3.3: Native Try Unwinding and Platform Meta Contracts

This gate closes the remaining language-level gaps before Gate 3 is considered
complete. It separates native `?` lowering from target-conditional metadata so
each contract can be validated independently.

#### Part 1: Native `?` operator and cleanup integration (ADR-0028)

- [x] Lower the `Result` discriminant check in Cranelift.
- [x] Continue with the `Ok` payload after successful unwrapping.
- [x] Write an `Err` payload into the caller-owned Result return storage safely.
- [x] Run deterministic LIFO cleanup for every active lexical scope on early
      return.
- [x] Preserve `abs` borrow release and `ins` loan restoration during early
      return.
- [x] Cover successful native unwrap execution.
- [x] Cover native error propagation with the original error payload.
- [x] Cover native resource drop during `?` early return.
- [x] Cover `?` with struct payloads and the sret ABI (ADR-0031).

#### Part 2: Target metadata contracts

- [x] Tokenize and parse `meta target(...)` declarations.
- [x] Validate supported target selectors and deterministic metadata syntax.
- [x] Filter declarations by the active target platform during semantic analysis.
- [x] Exclude declarations for non-matching targets before code generation.
- [x] Add native tests proving non-matching target code is not compiled.
- [x] Add diagnostics for malformed and unsupported target metadata.

### Gate 3.3 invariant

- [x] Native `?` propagation preserves Result layout, cleanup, and ownership.
- [x] Target metadata changes the compiled program only through explicit,
      deterministic target selection.

## Gate 3.5: Uniform Method Calls and I/O API Ergonomics

Implement ADR-0029 only after Gate 3 contracts are stable. Method syntax must
desugar to the existing role-qualified verb calls without hiding ownership.

- [x] Parse `receiver.verb(args...)` while preserving evaluation order.
- [x] Desugar the receiver into the first verb parameter.
- [x] Preserve explicit `erg`, `abs`, `dat`, and `ins` call-site roles.
- [x] Reject receiver/parameter type and role mismatches deterministically.
- [x] Route static method calls through direct monomorphized dispatch.
- [x] Keep dynamic role dispatch on the existing explicit ABI.
- [x] Migrate ergonomic `std::io` calls without changing their contracts.
- [x] Add parser, semantic, codegen, and native UFCS tests.
- [x] Update API documentation and editor tooling for method calls.

### Gate 3.5 invariant

- [x] Method syntax is only a source-level convenience over canonical verbs.
- [x] No implicit borrow, ownership transfer, vtable, or allocation is added.

## Gate 3.6: Primitive Type System and Representation Expansion

Stabilize Actus primitive numeric types and their native representations before
adding packed registers, MMIO contracts, or embedded HAL abstractions. This
gate defines the frontend and backend contracts for widths, literals, ranges,
and zero-sized values without implementing packed layouts yet.

### Lexer: Uniform Integer and Hex Tokenization

- [x] Lex `u1..u128` and `i1..i128` through one validated integer-type pattern.
- [x] Represent width-qualified integer types as
      `Token::IntType { signed: bool, width: u8 }`.
- [x] Lex hexadecimal literals with the `0x` prefix and validate their digit
      structure before parsing.

### Primitive Type Expansion

- [x] Add `f32` and `f64` IEEE 754 type syntax to lexing, parsing, and type
      references.
- [x] Add `Void` as a zero-sized unit type suitable for `Result[Void, E]`.
- [x] Preserve the distinction between type names and literal formats.

### Semantic Type Registry and Validation

- [x] Register the exact signedness and width for every `uN` and `iN` type.
- [x] Validate literal ranges at compile time; for example, reject values
      above `7` for `u3`.
- [x] Reject compile-time arithmetic overflow instead of silently truncating.
- [x] Produce deterministic diagnostics for unsupported widths and invalid
      numeric literals.

### Cranelift Native Lowering and Storage

- [x] Lower `u1..u8` and `i1..i8` to `I8` storage.
- [x] Lower `u9..u16` and `i9..i16` to `I16` storage.
- [x] Lower `u17..u32` and `i17..i32` to `I32` storage.
- [x] Lower `u33..u64` and `i33..i64` to `I64` storage.
- [x] Represent `u65..u128` and `i65..i128` with Cranelift `I128` where
      supported and deterministic 16-byte storage alignment.
- [x] Lower `f32` to `F32` and `f64` to `F64`.
- [x] Lower `Void` as a zero-byte, no-value evaluation result.

### Gate 3.6 invariant

- [x] Numeric types are no longer limited to `Int == I32`.
- [x] Numeric bounds and compile-time overflow are validated before native
      code generation.

## Gate 3.7: Stabilization and ABI Hardening

Freeze the Gate 3 compiler, runtime, and standard-library contracts before
introducing packed registers or MMIO-specific lowering in Gate 4. This gate
does not add new language syntax; it verifies that the existing contracts are
portable, documented, and safe to consume from native code.

### Documentation and ABI freeze

- [x] Document caller-allocated `sret` storage and aggregate payload copying in
      ADR-0031.
- [x] Document scalar, wide-integer, floating-point, and zero-sized `Void`
      return conventions in ADR-0031.
- [x] Align ADR-0028 with the implemented Result layout, cleanup, and early
      return contract.
- [x] Align ADR-0029 with the canonical call ABI and receiver evaluation order.
- [x] Record the distinction between fallible public APIs and private raw C
      runtime status codes.

### Error and standard-library contract audit

- [x] Verify that every fallible public `std::io` and `std::fs` operation
      returns `Result[T, IoError]`.
- [x] Keep raw host/C status values private to `unsafe extern "C"` declarations
      and convert them at the Actus library boundary.
- [x] Preserve infallible constructors and configuration verbs as direct value
      returns rather than manufacturing artificial I/O errors.
- [x] Verify UTF-8 validation, EOF handling, and failure propagation through
      one-shot and stream APIs.

### Native and tooling smoke matrix

- [x] Run native coverage for `tests/library/`, `tests/std_io.rs`,
      `tests/fs.rs`, and `tests/examples_cli.rs`.
- [x] Verify `examples/sensor_telemetry.act` through the Actus CLI.
- [x] Run Tree-sitter corpus tests for primitive types, hexadecimal literals,
      and ownership-role scopes.
- [x] Verify VS Code grammar packaging and installation for the Gate 3.6
      syntax surface.
- [x] Verify LSP primitive hover, completion, and semantic ownership tokens.

### Gate 3.7 invariant

- [x] The native ABI for structs, wide integers, floats, and `Void` is
      explicitly documented and covered by tests.
- [x] Public standard-library failures cannot expose raw host/C status codes.
- [x] Compiler and tooling checks are green before Gate 4 begins.

## Gate 4: Bare-Metal and Embedded HAL

Implement ADR-0026 for MMIO registers and deterministic protocol layouts. The
feature must remain usable on bare-metal targets and embedded microcontrollers
without host runtime services.

- [x] Reuse the `u1..u128` and `i1..i128` primitive type declarations from
      Gate 3.6.
- [x] Keep hexadecimal notation as a literal format only.
- [x] Reuse Gate 3.6 compile-time range and overflow validation.
- [x] Define `pack` with an explicit fixed backing integer.
- [x] Validate packed field widths, offsets, overlap, and total capacity.
- [x] Define deterministic endianness in the packed layout contract.
- [x] Integrate packed reads and writes with `erg` and `abs` access rules.
- [x] Lower shifts and masks deterministically through Cranelift IR.
- [x] Add native MMIO-style register layout tests.
- [x] Add network-frame layout tests.
- [x] Verify pack lowering has no allocator or host runtime dependency on
      bare-metal targets.

### Gate 4 invariant

- [x] Packed register layouts are deterministic across supported targets.
- [x] Width and overflow violations are rejected before native code generation.

## Gate 5: Advanced Memory and Scoped Arenas

Implement ADR-0030 for bounded cyclic data structures while retaining lexical
provenance and deterministic teardown.

- [x] Architecture & lexical/semantic contract: define `Arena[N]`, `place(...)`,
      and scope lifetime rules.
- [ ] Define `Arena[N]` capacity, alignment, and storage layout.
- [x] Provenance & escape analysis: bind arena roots and reject cross-arena or
      escaping references.
- [x] Add arena provenance metadata to semantic bindings and views.
- [x] Return objects from an arena through non-escaping `ins` construction.
- [x] Permit read-only traversal through `abs` references.
- [x] Reject references that escape the arena scope.
- [ ] Reject owner drop or move while arena-derived views are live.
- [x] Reject combinations of references from different arena roots.
- [x] Cranelift lowering & fast bump-pointer allocation.
- [x] Add native bump-pointer integrity and capacity-exhaustion tests.
- [x] Indirect-reference graph/tree native execution tests.
- [ ] Cyclical data structure native execution tests.
- [ ] Deterministic bulk teardown verification.
- [ ] Lower arena teardown to one deterministic bulk release operation.
- [ ] Add cyclic graph and tree native execution tests.
- [ ] Add exhaustion, nested-scope, and cross-arena negative tests.
- [ ] Verify operation without a host allocator on bare-metal targets.

### Gate 5 invariant

- [ ] Arena teardown is bounded and deterministic.
- [ ] Arena references cannot outlive or out-provenance their arena owner.

## Gate 6: Real Actus Programs, Portability, and Completion

Prove that the standard library and new language ergonomics are usable by
application code, then run the complete portability and quality matrix.

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
