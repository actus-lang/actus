# Phase 29: Neural Text Pipeline and Generic Facade Linking

## Status

In progress. Gate 29.0 is complete; no later gate in this phase is complete
until its implementation, negative
coverage, native evidence, and documentation are present in the repository.

## Objective

Make Actus capable of serving production systems that need safe text and
binary boundaries without embedding a particular neural application into the
compiler or standard library. The phase begins with a compiler correctness
fix: instantiated generic calls that cross nested module facades must be
discovered and emitted by native dependency collection. It then establishes
the hosted String/UTF-8 ownership boundary and the tooling/runtime contracts
that external applications can build on.

Tokenizer, vocabulary, sparse encoding, neural storage, training, and recall
belong to external application projects. They are not Actus compiler or
standard-library responsibilities and are intentionally excluded from this
phase.

## Architectural boundaries

- External applications own tokenizer, vocabulary, encoding, neural
  propagation, and learning policy. Those modules must consume the typed
  Actus text/buffer APIs rather than require compiler-specific hooks.
- Actus owns only the language, standard-library, runtime, ABI, ownership, and
  module-resolution contracts needed by those external applications.
- Compiler module facades remain the only public gateway. Private helpers may
  be emitted as native dependencies but must not become source-visible API.
- Application fixtures and neural data are external project contracts, not
  compiler implementation logic.

## Gate 29.0: Baseline and failure reproduction

- [x] Record the exact source shape for a root generic verb calling
      generic verbs exported through nested facades.
- [x] Verify the current state where `actus check --strict` and native
      object/executable emission run on the same fixture.
- [x] Record that the current compiler materializes the inferred specialization
      without a source-level manual specialization workaround.
- [x] Capture the module tree, facade exports, generic parameter bindings,
      native symbol identities, and dependency roots involved in the failure.
- [x] Add a minimal regression fixture independent of any commercial or
      domain-specific project.
- [x] Add a native baseline test so a future regression in the dependency
      closure is caught by check, object emission, executable linking, and run.

### Gate 29.0 evidence

- Checked-in fixture: `tests/fixtures/phase-29/nested-generic-facade/`.
- The fixture contains `main -> aie facade -> runtime facade -> implementation`
  and a generic `root[N]` that calls two generic helpers, including a nested
  `second[N] -> first[N]` call.
- `actus check --strict` succeeds; native object emission succeeds; native
  executable emission succeeds; the executable returns `24` deterministically.
- The current branch does not reproduce the previously reported undefined
  specialized-symbol failure. The baseline test records that fact and guards
  the working dependency path; Gate 29.1 remains responsible for proving the
  general fixed-point behavior and adding failure/visibility coverage.

## Gate 29.1: Transitive generic specialization through nested facades

- [x] During native dependency collection, traverse calls from concrete
      generic verbs and preserve each concrete call-site specialization.
- [x] Substitute type and const generic arguments before discovering child
      calls.
- [x] Resolve a called declaration through the existing facade namespace and
      retain deterministic specialized symbols.
- [x] Materialize distinct instances such as `root[4]` and `root[8]` exactly
      once, including their nested `first[N]` and `second[N]` dependencies.
- [x] Continue traversal to a fixed point until no new specialized dependency
      is discovered.
- [x] Preserve deterministic ordering of specialized instances and native
      symbols independent of filesystem traversal order.
- [x] Report missing generic arguments, invalid substitutions, and unresolved
      declarations as stable compiler diagnostics before linking.

### Gate 29.1 evidence

- `GenericInstance` now retains the source call span, and specialization uses
  `(callee, span)` identity when one caller has multiple concrete instances.
- The phase-29 fixture invokes `root[4]` and `root[8]` through the nested
  facade; each root instance calls both `first[N]` and `second[N]`, while
  `second[N]` calls `first[N]` transitively.
- The regression test verifies strict check, object emission, executable
  linking, deterministic repeated object bytes, and exit code `24`.
- Existing semantic tests cover missing and invalid generic arguments, while
  native dependency unit tests cover unresolved calls and stable call spans.
- Gate 29.1 is complete; visibility, ownership, cache-collision, and aggregate
  return contracts are tracked separately in Gate 29.2.

## Gate 29.2: Visibility, ownership, and specialization identity

- [x] Keep private child declarations inaccessible through direct source
      imports and public completion results.
- [x] Allow private declarations to enter the native dependency graph only
      when reached from an already-accepted public or test root.
- [x] Include module namespace, declaration identity, and concrete generic
      arguments in specialization cache keys.
- [x] Reject collisions between equal-looking generic symbols from different
      modules.
- [x] Ensure generic instances preserve ownership, `ins`, `abs`, and `dat`
      roles after substitution.
- [x] Ensure aggregate return types and return-slot requirements are registered
      transitively with the specialized dependency.
- [x] Verify that failed specialization cannot leave a partially registered
      native symbol available to a later build.

### Gate 29.2 evidence

- Accepted and rejected semantic visibility tests.
- Native symbol and cache-key collision tests across two module namespaces.
- Ownership regression tests for nested generic calls and aggregate returns.
- Existing facade visibility, private-dependency, ownership, aggregate-return,
  and failed-build cleanup tests provide the accepted/rejected evidence; the
  namespace-scoped generic symbol regression is in `tests/codegen_symbols.rs`.
- Gate 29.2 is complete. Gate 29.3 adds the cross-command and LSP parity
  verification.

## Gate 29.3: Compiler acceptance across all build paths

- [x] Make the nested generic facade fixture pass `actus check --strict`.
- [x] Make it pass `actus test --strict` through the package/module graph.
- [x] Emit a native object without undefined specialized symbols.
- [x] Link and execute a native binary with a deterministic success result.
- [x] Verify repeated object builds are byte-identical.
- [x] Verify zero-float IR auditing remains active and unchanged.
- [x] Verify LSP diagnostics, definition, hover, completion, and formatting use
      the same facade and specialization contract.
- [x] Verify `meta limitless("file")` affects only source limits and cannot
      remove dependency roots or alter specialization reachability.

### Gate 29.3 evidence

- One compiler regression test covering check, test, object, executable, and
  execution.
- Object reproducibility comparison and zero-float audit output.
- LSP and limitless-metadata regression evidence.
- The phase-29 fixture now has a package-level `meta test` that exercises both
  concrete generic instances through the imported facade. Its manifest enables
  the zero-float IR policy, and the CLI regression verifies check, test,
  object, executable, repeated object bytes, and runtime status together.
- Existing LSP facade, generic, formatting, and file-limitless tests cover the
  shared module/specialization contract across editor and CLI paths.
- Gate 29.3 is complete; later gates cover the text-facing String API.

## Gate 29.4: Safe UTF-8 String byte access

- [x] Define the public ownership contract for reading UTF-8 bytes from
      `String` without exposing raw pointers.
- [x] Provide a bounded, allocation-aware byte view or iterator with explicit
      `abs`/`ins` roles.
- [x] Define behavior for empty strings, multibyte UTF-8 sequences, invalid
      UTF-8, and index bounds.
- [x] Ensure byte access cannot create an escaping view or mutate an `abs`
      source.
- [x] Keep the existing String ABI and text output behavior compatible.
- [x] Add accepted and rejected semantic tests for ownership and UTF-8 cases.

### Gate 29.4 evidence

- Native tests for ASCII, multibyte UTF-8, empty input, and bounds failures.
- Allocation and ownership evidence for the selected byte-access contract.
- Documentation of whether iteration is byte-wise, scalar-value-wise, or both.

### Gate 29.4 evidence

- Added the hosted-only `std::string` facade with a documented `abs String`
  contract and typed `StringError` results. The API is byte-wise, not scalar-
  value iteration; multibyte code points therefore produce multiple validated
  UTF-8 bytes.
- Added private runtime bridges for checked length and indexed-byte access.
  They validate UTF-8 without allocation, reject null input and out-of-bounds
  indices deterministically, and preserve the existing null-terminated String
  ABI used by console output.
- Semantic tests verify the public facade and reject direct access to private
  bridge symbols. Native tests verify ASCII, multibyte UTF-8, empty strings,
  bounds errors, executable output status, and `verify_no_float_ir` builds.
- Runtime tests verify invalid UTF-8 and null-pointer statuses directly at the
  provider boundary. No String pointer or borrowed view escapes the call.
- Gate 29.4 is complete; Gate 29.5 covers constructing owned `Utf8Buffer`
  values from UTF-8 bytes.

## Gate 29.5: Constructing owned UTF-8 values from bytes

- [x] Define a typed builder or caller-owned output buffer for assembling an
      owned `Utf8Buffer` from validated UTF-8 bytes.
- [x] Reject invalid sequences deterministically; never silently replace or
      truncate malformed input.
- [x] Define capacity, length, overflow, and allocation behavior explicitly.
- [x] Ensure returned UTF-8 values have a valid ownership/drop contract.
- [x] Add native tests for round trips: `String -> bytes -> Utf8Buffer`.
- [x] Preserve exact bytes for embedded NUL and multibyte code points where the
      String contract permits them.
- [x] Keep raw C/runtime bridges private and typed at the Actus boundary.

### Gate 29.5 evidence

- `std::string` now exposes `utf8_from_string(abs text: String, dat storage:
  Buffer)` and `utf8_from_buffer(dat storage: Buffer)`. Both return an owned
  `Utf8Buffer` only after validation; rejected storage is deterministically
  dropped and translated to `StringError`.
- The hosted runtime copies String bytes into caller-owned capacity without
  allocation, rejects invalid metadata and invalid UTF-8, reports capacity
  exhaustion, and preserves exact length-delimited bytes including embedded
  NUL values. `utf8_length` and `utf8_byte_at` provide checked read access.
- Native tests cover String-to-owned-UTF-8 conversion, multibyte bytes, empty
  input, bounds failure, embedded NUL payloads, executable and object builds,
  and the package zero-float policy. Runtime tests cover exact copy counts and
  insufficient-capacity status; private C symbols remain inaccessible through
  semantic facade tests.
- The guide documents the distinction between legacy borrowed `String` and
  owned `Utf8Buffer`, including ownership roles and provider limitations.
- Gate 29.5 is complete; Gate 29.6 begins external-consumer integration
  readiness work.

## Gate 29.6: External consumer integration boundary

- [x] Document the supported `String`, `Utf8Buffer`, `Buffer`, and `Result`
      contracts for external application modules.
- [x] Verify that external consumers can import the public standard-library
      facade without accessing private runtime bridges.
- [x] Define the boundary between Actus-provided text storage and
      application-owned tokenizer, vocabulary, and encoding code.
- [x] Document hosted versus freestanding limitations for the text APIs.
- [x] Add a generic, non-domain-specific consumer fixture proving that an
      external module can read UTF-8 bytes and write owned bytes without
      compiler or application-specific hooks.

### Gate 29.6 evidence

- Public facade semantic tests and private-bridge rejection tests.
- Native object and executable evidence for the generic consumer fixture.
- Ownership, capacity, invalid-input, and zero-float evidence at the public
  boundary.
- Checked-in fixture: `tests/fixtures/phase-29/external-text-consumer/`.
- The fixture imports only `std::string`, transfers caller-owned `Buffer`
  storage into `Utf8Buffer`, reads a validated byte, and returns zero after
  strict check, object emission, executable linking, and execution.
- Existing `std::string` semantic/runtime/native tests provide the private
  bridge, malformed-input, capacity, ownership, and zero-float evidence.
- Gate 29.6 is complete; Gate 29.7 covers tooling and public API
  synchronization.

## Gate 29.7: Tooling and public API synchronization

- [x] Add formatter coverage for the public String and UTF-8 APIs.
- [x] Add LSP completion, hover, definition, diagnostics, and visibility tests
      through the String and UTF-8 facades.
- [x] Ensure `check`, `test`, object emission, executable emission, and LSP use
      the same module and native dependency graph.
- [x] Document every public type, verb, error, ownership role, and ABI boundary.
- [x] Update the coding guide, language guide, runtime documentation, API
      index, ADRs, and generic examples in the same logical change.
- [x] Verify no private runtime bridge leaks into public completion or
      documentation.

### Gate 29.7 evidence

- Formatter coverage confirms idempotent rendering of `std::string` imports,
  `Utf8Buffer` results, ownership roles, and named arguments.
- LSP coverage confirms diagnostics, hover, completion, definition, and
  formatting for the hosted standard-runtime String/UTF-8 facade.
- All existing facade, generic, visibility, native, and strict workflow tests
  remain green; private runtime symbols are not exposed.
- Gate 29.7 is complete; Gate 29.8 covers resource, determinism, and safety
  acceptance.

## Gate 29.8: Resource, determinism, and safety acceptance

- [x] Add no-allocation tests for APIs that promise caller-owned storage.
- [x] Add malformed-input, capacity-exhaustion, stale-handle, and bounds
      rejection tests.
- [x] Verify deterministic cleanup on every error and early-return path.
- [x] Verify zero-float IR for integer-only text and buffer operations.
- [x] Add reproducible object and executable build checks.
- [x] Confirm no unsafe bridge is exposed without a typed, documented boundary.

### Gate 29.8 evidence

- `std::string` native and runtime tests cover malformed UTF-8, invalid
  storage, capacity exhaustion, bounds rejection, and owned-buffer cleanup.
- The external-consumer fixture uses caller-owned storage and verifies
  zero-float native output plus byte-identical repeated object emission.
- Semantic visibility tests keep unsafe runtime bridges private while public
  APIs return typed `Result` errors.
- Gate 29.8 is complete; Gate 29.9 covers end-to-end compiler/runtime
  acceptance.

## Gate 29.9: End-to-end compiler and runtime acceptance

- [x] `actus check --strict` passes for the complete generic text consumer
      fixture.
- [x] `actus test --strict` passes all String, UTF-8, facade, and linker
      regression tests.
- [x] Native object and executable builds pass with no undefined specialized
      facade symbols.
- [x] The generic consumer example executes successfully without
      application-specific neural structures.
- [x] Zero-float, allocation, determinism, visibility, source-limit, and
      documentation checks pass at the Actus boundary.
- [x] The final API and evidence are reviewed against every earlier gate; no
      checkbox is closed from a source-level check alone.

### Gate 29.9 evidence

- The external-consumer fixture now passes `actus check --strict`,
  `actus test --strict`, native object emission, native executable linking,
  and executable execution.
- The package test runner exercises the same `std::string` facade and typed
  `Utf8Buffer` boundary as the executable entry point.
- Full repository checks pass, including formatter, compiler checks, clippy,
  all-target tests, source limits, documentation validation, and diff checks.
- Gate 29.9 is complete; Phase 29 is complete for the Actus compiler/runtime
  boundary.

## Gate 29.10: Cross-module root generic specialization and internal loading

- [x] Preserve concrete generic instances discovered in a root verb when the
      called generic declaration is exported through one or more nested
      module facades.
- [x] Propagate the fixed-point generic instance set to every imported native
      object that may define a specialized dependency.
- [x] Resolve nested child modules through their admitted parent facade during
      compiler-internal aggregation without weakening the external
      facade-bypass diagnostic.
- [x] Keep direct child-module imports rejected while allowing the compiler's
      own object planner to load an already-admitted child implementation.
- [x] Emit deterministic specialized symbols and link the root object against
      the imported object without manual concrete calls in source code.
- [x] Add a neutral regression test covering a root generic wrapper, a
      multi-level facade chain, a generic imported verb, object/executable
      emission, linking, and deterministic execution.

### Gate 29.10 evidence

- `tests/cli_workflow.rs::native_build_specializes_root_generic_calls_through_nested_facades`
  builds and executes a neutral `device -> runtime -> implementation` module
  tree; the root generic wrapper reaches the imported generic verb and the
  executable returns `4`.
- Native emission now expands the transitive generic instance set before
  imported module objects are emitted, so specialized dependencies are not
  left as undefined linker symbols.
- Compiler-internal aggregation uses a separate admitted-child resolution
  path. Public resolver calls continue to reject facade bypasses, and the
  existing resolver, module, standard-library, and native regression suites
  remain green.
- Gate 29.10 is complete; no source-level workaround or domain-specific
  compiler dependency was introduced.

## Gate 29.11: Shared external ABI symbols across standard-library facades

- [x] Treat compatible external declarations with the same native symbol name
      as one shared ABI contract during module aggregation.
- [x] Compare ABI, generic signature, parameter types, ownership roles,
      dispatch modes, and return access before allowing deduplication.
- [x] Preserve deterministic `E1110` diagnostics for incompatible external
      declarations that reuse one native symbol name.
- [x] Keep native bridge declarations private to their owning facade while
      allowing multiple public facades to reference the compatible bridge.
- [x] Do not rename or duplicate the runtime ABI symbol as a source-level
      workaround.
- [x] Add accepted and rejected compiler regression tests for compatible and
      incompatible external declarations across modules.
- [x] Add an end-to-end hosted test importing both `std::io` and
      `std::string`, covering strict check, object emission, executable
      linking, and deterministic execution.

### Gate 29.11 evidence

- Compatible declarations sharing one external buffer-length ABI are accepted
  and emitted as references to the same native symbol.
- Incompatible ownership contracts for the same external symbol are rejected
  with `E1110` before native emission.
- The combined standard-library fixture passes strict checking, object build,
  executable build, and execution with exit code `0`.
- Full compiler tests, clippy, source-limit validation, and diff validation
  pass after the change.
- Gate 29.11 is complete.

## Non-goals

- This phase does not promise human-level intelligence, semantic truth, or
  general language understanding.
- This phase does not implement tokenizers, vocabularies, sparse encoders,
  neural engines, training, or recall APIs.
- This phase does not expose application-specific neural hardware layout as a
  general Actus API.
- This phase does not add a desktop UI, hosted product integration, or a
  commercial application.
- This phase does not hide allocation, scheduler, persistence, or hardware
  requirements behind a convenient name.

## Completion criteria

Phase 29 is complete only when nested generic calls through module facades are
correctly specialized and linked without manual concrete calls, String/UTF-8
boundaries are typed and tested, external consumers have a documented and
verified integration boundary, and all compiler/tooling/native/resource gates
have direct evidence. Application tokenization, encoding, and neural behavior
are validated in their own repositories.
