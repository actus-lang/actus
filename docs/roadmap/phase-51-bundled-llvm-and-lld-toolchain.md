# Phase 51 — Bundled LLVM and lld toolchain

## Status

Proposed. The initial T1 checkpoint drafts ADR-0105 and does not change build,
bootstrap, compiler, linker, or release code.

## Objective

Ship Actus with a reproducible, checksum-verified LLVM 23.1.2 toolchain so
that normal users and clean CI environments do not require a separately
installed LLVM development package or lld. Preserve the explicit backend,
target, ownership, deterministic, zero-float, ABI, and stack-proof contracts
while adding a product-quality distribution path. The primary freestanding
target is `thumbv7em-none-eabihf` for Cortex-M4F-class devices; the secondary
target is `thumbv7em-none-eabi` for soft-float/no-FPU devices.

## Governing decision

ADR-0105 defines the proposed linkage, version, archive, bootstrap, identity,
lld, licensing, and test contracts. Implementation must not begin until the
ADR is reviewed and accepted.

## Work sequence

### 51.1 — Architecture and release contract

- [x] Create ADR-0105 with the static/shared linkage comparison and initial
      static-link recommendation.
- [x] Pin LLVM 23.1.2 to source commit
      `85ac560262434c9ccfc0c183ec22d4138ed647fb`.
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
- [x] Define `thumbv7em-none-eabihf` as the primary and
      `thumbv7em-none-eabi` as the secondary freestanding target.
- [x] Define separate user and contributor/development artifacts.
- [x] Define the glibc 2.28 Linux baseline and Windows `/MD` runtime policy.
- [x] Define `unverified` system-override behavior and strict-build rejection.
- [x] Record the official LLVM 23.1.2 archive as a transitional fallback and
      define its provenance trade-off.
- [ ] Review and accept ADR-0105.

### 51.2 — Reproducible LLVM archive builder

T2 is limited to contributor/development archives named
`llvm-dev-23.1.2-<host-triple>`. The user-facing `actus-<version>-<host-triple>`
archive requires an Actus binary built with the bundled LLVM toolchain and is
tracked in 51.7 after the compiler integration work. Builtin helper-registry
integration is tracked in 51.5 and is linked to ADR-0103 S3.

- [x] Add CI jobs that build LLVM 23.1.2 from the pinned source revision for
      Linux x86-64.
- [x] Restrict the Linux build to X86, ARM, and AArch64.
- [x] Disable zlib, zstd, libxml2, and terminfo unless a reviewed dependency
      exception is recorded.
- [x] Build the Linux archive in an image with glibc 2.28 as the compatibility
      baseline.
- [ ] Configure Windows LLVM with `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL`
      so it matches Rust's `/MD` runtime contract.
- [x] Produce the Linux `llvm-dev-23.1.2` contributor archive with a
      repository-generated SHA-256 entry.
- [x] Include target-specific compiler-rt builtins in the Linux contributor
      archive at `lib/actus/targets/<target>/libclang_rt.builtins.a` for both
      `thumbv7em-none-eabihf` and `thumbv7em-none-eabi`.
- [x] Use LLVM distribution components and `install-distribution` so Clang is
      built but not shipped in the Linux contributor archive; ship only
      headers, LLVM libraries, `llvm-config`, lld, builtins, and inspection
      tools.
- [x] Keep the CMake cache free of build-host-specific compiler launchers;
      pass ccache through workflow `-D` arguments.
- [x] Publish the Linux contributor archive with the compiler headers, tools,
      lld, manifest, and licenses needed by the supported build paths.
- [x] Verify the Linux archive contents and identity before release publication.

#### Linux T2 evidence

The following evidence was collected from the two successful workflow runs:

- [Run #5](https://github.com/actus-lang/actus/actions/runs/42146115312)
  built the archive without a reusable compiler cache. The full build ran
  from `2026-11-06T01:51:26Z` to `2026-11-06T03:21:30Z`, approximately
  **1h 30m 04s**. Its ccache report was: 3,315 misses, 0 direct hits, 0
  preprocessed hits, 0.00% hit rate, 9,522 files, and 562.5 MB used.
- [Run #6](https://github.com/actus-lang/actus/actions/runs/42158469005)
  rebuilt the same archive with the restored compiler cache. The full build
  ran from `2026-11-06T03:49:43Z` to `2026-11-06T03:58:15Z`, approximately
  **8m 32s**. Its ccache report was: 2,846 direct hits, 469 preprocessed
  hits, 0 misses, 100.00% hit rate, 9,522 files, and 562.5 MB used.
- Both runs produced the same archive size: **184,020,984 bytes**.
- Both runs produced the same archive SHA-256:
  `6a73055d2c36b10b14ccd0fba354bc5ad95f8ffd71e1c1f3a69d22eaf7b7a3e7`.
- `resolve_builtin` found exactly one approved source path for each target:
  - `lib/clang/23/lib/thumbv7em-unknown-none-eabi/libclang_rt.builtins.a`
  - `lib/clang/23/lib/thumbv7em-unknown-none-eabihf/libclang_rt.builtins.a`
  These were staged into the public archive paths:
  - `lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a`
  - `lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a`
- The clean verification job passed for both runs with the exact output:
  `verified llvm-dev-23.1.2-x86_64-unknown-linux-gnu.tar.xz in a clean glibc
  2.28 container`.
  This verification checked LLVM 23.1.2, the X86/ARM/AArch64 targets, bundled
  lld, both builtin archives and `__aeabi_uldivmod`, forbidden shared-library
  dependencies, the glibc 2.28 ceiling, the manifest identity, and the
  integer-only Thumb fixture's absence of VFP instructions.
- Both runs used LLVM source revision
  `85ac560262434c9ccfc0c183ec22d4138ed647fb` and image digest
  `sha256:7e41b1c7fd64dfc69703a1124b43c404382973ee06ec5d764b85c34d2221fded`.

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

- [ ] Integrate compiler-rt builtin helper symbols and transitive stack claims
      with the ADR-0103 S3 helper registry.
- [ ] Package a matching `ld.lld` executable at `lib/actus/bin/ld.lld` and
      record it in the identity manifest without shadowing system `PATH` tools.
- [ ] Select bundled lld only for the primary `thumbv7em-none-eabihf` and
      secondary `thumbv7em-none-eabi` freestanding paths.
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
- [ ] Verify `llvm-config` and `ld.lld` have no libstdc++, zlib, zstd, libxml2,
      or libtinfo dependency and require no GLIBC symbol newer than 2.28.
- [ ] Disassemble an integer-only `thumbv7em-none-eabihf` fixture and reject
      any VFP instruction, including `vadd`, `vmov`, `vldr`, `vstr`, or
      `vpush`, despite the ABI permitting FPU instructions.
- [ ] In T4/T5, disassemble real Actus-generated
      `thumbv7em-none-eabihf` objects and apply the same zero-float VFP
      rejection; the LLVM IR fixture above is only the T2 toolchain smoke
      test.
- [ ] Test repeated bootstrap and repeated builds for the required byte-level
      determinism.
- [ ] Test archive relocation and a clean user installation path.
- [ ] Record exact commands, target triples, profiles, toolchain checksums, and
      complete failures in the roadmap.

### 51.7 — Release packaging and notices

- [ ] Produce the user-facing `actus-<version>-<host-triple>` archive after
      T4 provides an Actus binary built with the bundled LLVM toolchain. It
      must include the compiler, runtime, standard library, target builtins,
      identity metadata, licenses, and supported link tools, but no development
      headers or `llvm-config` prefix.
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

