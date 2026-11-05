# Phase 51 — Bundled LLVM and lld toolchain

## Status

Proposed. The initial T1 checkpoint drafts ADR-0105 and does not change build,
bootstrap, compiler, linker, or release code.

## Objective

Ship Actus with a reproducible, checksum-verified LLVM 23.1.2 toolchain so
that normal users and clean CI environments do not require a separately
installed LLVM development package or lld. Preserve the explicit backend,
target, ownership, deterministic, zero-float, ABI, and stack-proof contracts
while adding a product-quality distribution path.

## Governing decision

ADR-0105 defines the proposed linkage, version, archive, bootstrap, identity,
lld, licensing, and test contracts. Implementation must not begin until the
ADR is reviewed and accepted.

## Work sequence

### 51.1 — Architecture and release contract

- [x] Create ADR-0105 with the static/shared linkage comparison and initial
      static-link recommendation.
- [x] Pin the initial LLVM release line to LLVM 23.1.2 in the proposal.
- [x] Define target selection, optional dependency policy, and the Linux C++
      runtime policy.
- [x] Define toolchain identity, checksum, archive, license, and relocation
      requirements.
- [x] Define contributor bootstrap behavior and explicit system override rules.
- [x] Define the separate lld executable boundary and its initial freestanding
      target scope.
- [x] Define clean-environment and release-archive acceptance evidence.
- [x] Define freestanding compiler-rt builtin and ADR-0103 helper-registry
      requirements.
- [x] Define separate user and contributor/development artifacts.
- [x] Define the glibc 2.28 Linux baseline and Windows `/MD` runtime policy.
- [x] Define `unverified` system-override behavior and strict-build rejection.
- [x] Record the official LLVM 23.1.2 archive as a transitional fallback and
      define its provenance trade-off.
- [ ] Review and accept ADR-0105.

### 51.2 — Reproducible LLVM archive builder

- [ ] Add CI jobs that build LLVM 23.1.2 from the pinned source revision.
- [ ] Restrict the build to X86, ARM, and AArch64.
- [ ] Disable zlib, zstd, libxml2, and terminfo unless a reviewed dependency
      exception is recorded.
- [ ] Build in a Linux image with glibc 2.28 as the compatibility baseline.
- [ ] Configure Windows LLVM with `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL`
      so it matches Rust's `/MD` runtime contract.
- [ ] Produce separate user and `llvm-dev-23.1.2` archives with
      repository-tracked SHA-256 entries.
- [ ] Include target-specific compiler-rt builtins in both artifacts at
      `lib/actus/targets/<target>/libclang_rt.builtins.a` and register their
      helper symbols and stack claims through the ADR-0103 helper registry.
- [ ] Publish the compiler, headers, tools, lld, manifest, and licenses needed
      by the supported build paths.
- [ ] Verify archive contents and identity before release publication.

### 51.3 — Contributor bootstrap and Cargo discovery

- [ ] Add idempotent Unix and Windows bootstrap scripts.
- [ ] Install verified archives under `.actus-toolchain/llvm-23.1.2/`.
- [ ] Configure `LLVM_SYS_231_PREFIX` through repository-local Cargo
      configuration with relative environment lookup.
- [ ] Reject corrupted, mismatched, or incomplete archives before extraction or
      compilation.
- [ ] Warn clearly when an explicit system LLVM override is used.
- [ ] Mark system-override identity as `unverified` and reject it from
      `--strict` builds and stack-limit proof acceptance.
- [ ] Prove bootstrap does not modify shell profiles or global configuration.

### 51.4 — Compiler identity and backend integration

- [ ] Add LLVM version, archive checksum, build revision, target, profile, and
      linker identity to `actus --version`.
- [ ] Include the same identity in stack-limit reports and future structured
      MCP responses.
- [ ] Preserve artifact invalidation when backend, target, profile, or LLVM
      identity changes.
- [ ] Keep LLVM-specific types and discovery inside the LLVM backend boundary.
- [ ] Verify static-link output and record binary-size and link-time evidence.

### 51.5 — Bundled lld target contract

- [ ] Package a matching `ld.lld` executable at `lib/actus/bin/ld.lld` and
      record it in the identity manifest without shadowing system `PATH` tools.
- [ ] Select bundled lld only for the supported freestanding
      `thumbv7em-none-eabi` path.
- [ ] Keep hosted Linux on its documented C linker/libc contract and preserve
      explicit `ACTUS_LINKER` overrides.
- [ ] Reject linker version or target mismatches with stable diagnostics.
- [ ] Add object, link, and execution evidence for the freestanding path.

### 51.6 — Clean-environment and determinism evidence

- [ ] Run compiler, standard-library, backend, ABI, stack-report, zero-float,
      and ownership tests without a system LLVM installation.
- [ ] Test missing, corrupted, and wrong-version archives.
- [ ] Test the distinct user archive and contributor `llvm-dev` archive from
      clean environments.
- [ ] Test repeated bootstrap and repeated builds for the required byte-level
      determinism.
- [ ] Test archive relocation and a clean user installation path.
- [ ] Record exact commands, target triples, profiles, toolchain checksums, and
      complete failures in the roadmap.

### 51.7 — Release packaging and notices

- [ ] Produce relocatable Unix and Windows distribution archives.
- [ ] Include `actus`, standard library/runtime files, identity metadata, and
      third-party notices.
- [ ] Verify LLVM/lld Apache-2.0 WITH LLVM-exception notices against the pinned
      source revision.
- [ ] Publish checksums and a release manifest beside every archive.
- [ ] Document installation, rollback, explicit system override, and removal.

## T1 checkpoint evidence

The T1 work is complete when:

- the working branch contains ADR-0105;
- this roadmap records the ordered implementation gates;
- no compiler, build script, bootstrap script, linker, dependency, or release
  archive has been changed; and
- the ADR is explicitly awaiting review before T2 begins.

## Non-goals

- LLVM IR release optimization, which remains Phase 50;
- changing Actus language semantics, ownership, ABI, or serialized formats;
- making Cranelift a silent fallback;
- supporting every host linker through one abstraction; and
- embedding lld through its internal C++ library API in the first release.

