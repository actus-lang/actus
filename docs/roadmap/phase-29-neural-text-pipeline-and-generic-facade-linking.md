# Phase 29: Neural Text Pipeline and Generic Facade Linking

## Status

In progress. Gate 29.0 is complete; no later gate in this phase is complete
until its implementation, negative
coverage, native evidence, and documentation are present in the repository.

## Objective

Make Actus capable of expressing a production-quality text-to-neural pipeline
without forcing application code to reproduce the internal AIE storage layout.
The phase begins with a compiler correctness fix: instantiated generic calls
that cross nested module facades must be discovered and emitted by native
dependency collection. Only after that foundation is proven will the language
surface be extended for safe UTF-8 access, byte-to-string construction,
tokenization, sparse encodings, and high-level phrase training/recall APIs.

The intended application surface is:

```act
erg phrase: String = "Hello Giorgi";
train_phrase(fabric: ins fabric, phrase: abs phrase);

erg result: String = recall_phrase(fabric: ins fabric, input: abs phrase);
println(text: abs result);
```

The internal neural representation may remain explicit and hardware-oriented
inside its implementation modules. That representation must not leak into the
text-facing API.

## Architectural boundaries

```text
src/tokenizer/  -> String/UTF-8 input into tokens
src/encoding/   -> token identifiers into fixed-width sparse codes
src/aie/        -> propagation, plasticity, storage, and recall
```

- `src/tokenizer/` owns vocabulary lookup, UTF-8 token boundaries, token IDs,
  and decoding policy. It must not know AIE node layout.
- `src/encoding/` owns token-ID-to-code mapping and 128-bit sparse-code
  operations. It must not own phrase storage or module resolution.
- `src/aie/` owns neural propagation and learning. It must not parse UTF-8 or
  hardcode a user-facing phrase vocabulary.
- Compiler module facades remain the only public gateway. Private helpers may
  be emitted as native dependencies but must not become source-visible API.
- Static training fixtures are data contracts, not compiler implementation
  logic. They must be deterministic, versioned, and independently testable.

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

- [ ] Define the public ownership contract for reading UTF-8 bytes from
      `String` without exposing raw pointers.
- [ ] Provide a bounded, allocation-aware byte view or iterator with explicit
      `abs`/`ins` roles.
- [ ] Define behavior for empty strings, multibyte UTF-8 sequences, invalid
      UTF-8, and index bounds.
- [ ] Ensure byte access cannot create an escaping view or mutate an `abs`
      source.
- [ ] Keep the existing String ABI and text output behavior compatible.
- [ ] Add accepted and rejected semantic tests for ownership and UTF-8 cases.

### Gate 29.4 evidence

- Native tests for ASCII, multibyte UTF-8, empty input, and bounds failures.
- Allocation and ownership evidence for the selected byte-access contract.
- Documentation of whether iteration is byte-wise, scalar-value-wise, or both.

## Gate 29.5: Constructing String values from UTF-8 bytes

- [ ] Define a typed builder or caller-owned output buffer for assembling a
      `String` from validated UTF-8 bytes.
- [ ] Reject invalid sequences deterministically; never silently replace or
      truncate malformed input.
- [ ] Define capacity, length, overflow, and allocation behavior explicitly.
- [ ] Ensure returned strings have a valid ownership/drop contract.
- [ ] Add native tests for round trips: `String -> bytes -> String`.
- [ ] Preserve exact bytes for embedded NUL and multibyte code points where the
      String contract permits them.
- [ ] Keep raw C/runtime bridges private and typed at the Actus boundary.

### Gate 29.5 evidence

- Accepted/rejected UTF-8 fixtures with exact output bytes.
- Native executable and object evidence, including failure paths.
- No-allocation evidence where the caller supplies bounded storage.

## Gate 29.6: Tokenizer module and vocabulary contract

- [ ] Create `src/tokenizer/` with a canonical facade and responsibility-sized
      sibling modules.
- [ ] Define token ID width, unknown-token behavior, vocabulary ownership, and
      deterministic ordering.
- [ ] Separate UTF-8 decoding from vocabulary lookup.
- [ ] Define tokenization of whitespace, punctuation, repeated separators,
      multibyte characters, and unknown sequences.
- [ ] Make vocabulary data manifest- or fixture-driven rather than hardcoded
      inside compiler code.
- [ ] Add encode/decode round-trip tests and rejected malformed-input tests.
- [ ] Keep tokenizer API independent of AIE node and axon structures.

### Gate 29.6 evidence

- Deterministic tokenizer fixtures with expected token IDs and reconstructed
  text.
- Static vocabulary fixture checksum and version identity.
- Native tests for bounded storage and failure behavior.

## Gate 29.7: Sparse 128-bit encoding module

- [ ] Create `src/encoding/` with a canonical facade.
- [ ] Define the exact 128-bit code representation and active-bit invariant.
- [ ] Define deterministic token-ID-to-pattern mapping and collision behavior.
- [ ] Provide integer-only bind, bundle, permutation, overlap, and population
      count operations appropriate to the selected representation.
- [ ] Keep bit patterns and mapping policy outside user-facing phrase code.
- [ ] Add tests for determinism, orthogonality/collision expectations, zero
      codes, maximum IDs, and unsupported IDs.
- [ ] Verify no floating-point instructions, hidden allocation, or unstable
      platform-dependent bit ordering.

### Gate 29.7 evidence

- Golden vectors for every encoding operation.
- Native IR zero-float audit and deterministic object comparison.
- Benchmark measurements separated from correctness assertions.

## Gate 29.8: High-level phrase training and recall API

- [ ] Define a public `train_phrase` API that accepts `String` and an explicit
      exclusive `ins` fabric/storage contract.
- [ ] Define a public `recall_phrase` API with typed success and failure
      behavior, including unknown input and insufficient-capacity cases.
- [ ] Keep 64-byte minicolumn layout, axon slots, free-list, and plasticity
      rules private to the AIE implementation facade.
- [ ] Ensure training and recall do not require callers to write one verb per
      character or hand-assign 128-bit patterns.
- [ ] Define repeat training, conflicting phrases, capacity exhaustion,
      deterministic replay, and persistence behavior.
- [ ] Add end-to-end tests for `String -> train -> recall -> String`.

### Gate 29.8 evidence

- A readable application fixture using only the high-level phrase API.
- Exact success/failure outputs and deterministic replay assertions.
- Ownership, allocation, and native execution evidence for training and recall.

## Gate 29.9: Static fixture and benchmark data separation

- [ ] Move phrase/token/pattern data into a versioned static fixture format.
- [ ] Remove hand-written per-character benchmark setup from executable
      application code.
- [ ] Define fixture schema, version, checksum, ordering, and compatibility
      policy.
- [ ] Separate correctness fixtures from performance workloads.
- [ ] Keep benchmark output reproducible and label machine/scheduler/compiler
      conditions without presenting timing as a universal guarantee.
- [ ] Add fixture corruption, truncation, version mismatch, and duplicate-entry
      rejection tests.

### Gate 29.9 evidence

- Checked-in static fixture and schema documentation.
- Loader tests for valid and invalid fixtures.
- A benchmark that consumes fixture data without embedding its contents in
  source-level control flow.

## Gate 29.10: Tooling and public API synchronization

- [ ] Add formatter coverage for tokenizer, encoding, and phrase APIs.
- [ ] Add LSP completion, hover, definition, diagnostics, and visibility tests
      through all new facades.
- [ ] Ensure `check`, `test`, object emission, executable emission, and LSP use
      the same module and native dependency graph.
- [ ] Document every public type, verb, error, ownership role, and ABI boundary.
- [ ] Update the coding guide, language guide, runtime documentation, API
      index, ADRs, and examples in the same logical change.
- [ ] Verify no private AIE layout or raw bridge leaks into public completion or
      documentation.

## Gate 29.11: Resource, determinism, and safety acceptance

- [ ] Define memory ownership and bounded-storage behavior for tokenizer,
      encoding, training, and recall.
- [ ] Add no-allocation tests for APIs that promise caller-owned storage.
- [ ] Add malformed-input, capacity-exhaustion, stale-handle, and bounds
      rejection tests.
- [ ] Verify deterministic cleanup on every error and early-return path.
- [ ] Verify zero-float IR for integer-only encoding and neural operations.
- [ ] Add reproducible object and executable build checks.
- [ ] Confirm no unsafe bridge is exposed without a typed, documented boundary.

## Gate 29.12: End-to-end production acceptance

- [ ] `actus check --strict` passes for the complete text pipeline project.
- [ ] `actus test --strict` passes all tokenizer, encoding, AIE, fixture, and
      linker regression tests.
- [ ] Native object and executable builds pass with no undefined specialized
      facade symbols.
- [ ] A readable phrase training and recall example executes successfully.
- [ ] The example does not contain per-character pattern assignments or manual
      internal layout construction.
- [ ] Zero-float, allocation, determinism, visibility, source-limit, and
      documentation checks pass.
- [ ] The final API and evidence are reviewed against every earlier gate; no
      checkbox is closed from a source-level check alone.

## Non-goals

- This phase does not promise human-level intelligence, semantic truth, or
  general language understanding.
- This phase does not require BPE, a specific tokenizer algorithm, or a neural
  learning law before the corresponding contract is reviewed.
- This phase does not expose AIE hardware layout as a general application API.
- This phase does not add a desktop UI, hosted product integration, or a
  commercial application.
- This phase does not hide allocation, scheduler, persistence, or hardware
  requirements behind a convenient name.

## Completion criteria

Phase 29 is complete only when nested generic calls through module facades are
correctly specialized and linked without manual concrete calls, String/UTF-8
boundaries are typed and tested, tokenizer/encoding/AIE responsibilities are
separate, phrase training and recall use a readable public API, fixtures are
versioned and deterministic, and all compiler/tooling/native/resource gates
have direct evidence.
