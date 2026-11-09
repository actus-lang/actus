# Phase 51 — Bundled LLVM and lld toolchain

T1–T7 mapping: T1 = 51.1, T2 = 51.2, T3 = 51.3, T4 = 51.4, T5 = 51.6,
T6 = 51.5, and T7 = 51.7.

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
- [x] Build the macOS arm64 `llvm-dev-23.1.2-aarch64-apple-darwin` archive on
      the pinned `macos-15`/Xcode 16.4 image with the system libc++ policy.
- [x] Verify the macOS archive through archive-prefix resolution, deployment
      target, system-library dependency, target, builtin, lld, manifest, and
      zero-float checks.
- [x] Build and verify the Windows `llvm-dev-23.1.2-x86_64-pc-windows-msvc`
      archive with the pinned MSVC `/MD` contract, toolset identity, builtin
      provenance, and archive-prefix dependency checks.

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

#### macOS and Windows T2 evidence

Run [#14](https://github.com/actus-lang/actus/actions/runs/42330707485)
completed successfully on all Linux, macOS, and Windows build and verification
jobs.

- macOS arm64 ran on `macos-15` arm64 with Xcode 16.4 and
  `MACOSX_DEPLOYMENT_TARGET=13.0`. The build job ran from
  `2026-11-07T02:44:26Z` to `2026-11-07T03:47:35Z`, approximately
  **1h 03m 09s**. It produced
  `llvm-dev-23.1.2-aarch64-apple-darwin.tar.xz`, **118,078,516 bytes**, with
  SHA-256
  `d9ac829b0a1ee4e36d8993f28f4494120b43c1cfc82ec61076f83bb63860c8f4`.
- macOS `resolve_builtin` found the approved source paths
  `lib/clang/23/lib/thumbv7em-unknown-none-eabihf/libclang_rt.builtins.a` and
  `lib/clang/23/lib/thumbv7em-unknown-none-eabi/libclang_rt.builtins.a`.
  They were staged as
  `lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a` and
  `lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a`.
  Verification passed with archive-prefix, deployment-target, system-library,
  target, builtin, lld, manifest, and zero-float checks.
- The macOS run recorded the pre-PCH-cache baseline: 3,322 compiler calls,
  19 cacheable calls, 0 hits, 19 misses, 3,303 uncacheable calls, and a
  0.02% cache-size utilization. The uncacheable-call cause is tracked
  separately: LLVM's enabled precompiled headers are not cacheable by ccache.
- Windows x86-64 ran on `windows-2022` with MSVC toolset `14.44.35207`
  (`VCToolsVersion`) and the `/MD` contract. The build job ran from
  `2026-11-07T02:44:26Z` to `2026-11-07T03:00:11Z`, approximately
  **15m 45s**. It produced
  `llvm-dev-23.1.2-x86_64-pc-windows-msvc.zip`, **1,194,964,192 bytes**, with
  SHA-256
  `92fd65fb609f54906437de78edb45a58534f7ff20793c5d41ac821ce47347a01`.
- Windows staged the two builtins as
  `lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a` and
  `lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a`; their source
  paths were
  `lib/clang/23/lib/thumbv7em-unknown-none-eabihf/libclang_rt.builtins.a` and
  `lib/clang/23/lib/thumbv7em-unknown-none-eabi/libclang_rt.builtins.a`.
  Verification passed with archive-prefix, MSVC dependency, builtin, and
  zero-float checks.

### 51.3 — Contributor bootstrap and Cargo discovery

- [x] Add idempotent Unix and Windows bootstrap scripts.
- [x] Install verified archives under `.actus-toolchain/llvm-23.1.2/`.
- [x] Configure `LLVM_SYS_231_PREFIX` through repository-local Cargo
      configuration with relative environment lookup.
- [x] Reject corrupted, mismatched, or incomplete archives before extraction or
      compilation.
- [x] Warn clearly when an explicit system LLVM override is used.
- [x] Mark system-override identity as `unverified` and reject it from
      `--strict` LLVM builds and stack-limit proof acceptance with E1806.
- [x] Prove bootstrap does not modify shell profiles or global configuration.

T3 release evidence:

- Release: [llvm-23.1.2-actus-1](https://github.com/actus-lang/actus/releases/tag/llvm-23.1.2-actus-1), a prerelease development toolchain rather than an Actus release.
- Publish/verification run: [42417431157](https://github.com/actus-lang/actus/actions/runs/42417431157). Linux, macOS arm64, and Windows verification passed; the workflow's direct manifest push was rejected by branch protection and was removed. The manifest is maintained through normal pull requests.
- `x86_64-unknown-linux-gnu`: 184026264 bytes, SHA-256 `af5e2c36266e955b3814ca48a49b128a9edb93bc9d93c782fa2d30789a2de679`.
- `aarch64-apple-darwin`: 118102460 bytes, SHA-256 `0572360506bc40e0203cb341ad8e400fc138b62c233ed14fec2602fbf0fdaa5b`.
- `x86_64-pc-windows-msvc`: 1195012852 bytes, SHA-256 `85876a42e66d0ca4720a5884256cbe5ba1da1e3787190c1791d1b7bad84669b9`.
- Each archive contains `licenses/LLVM-LICENSE.TXT` and
  `licenses/THIRD-PARTY-NOTICES.txt`.

The archive-size investigation found that the published Windows ZIP used
`ZIP_STORED` for all 2,653 entries, so its 1,195,012,852-byte size was nearly
the same as the extracted payload. The packager now uses deterministic
DEFLATE level 9 for every ZIP entry. The next archive verification also keeps
the format as ZIP and enforces upper bounds derived from the rebuilt contents:
196,686,557 bytes for Linux, 125,088,711 bytes for macOS arm64, and 332,593,539
bytes for Windows. These values are acceptance ceilings, not replacement
release identities; the published `actus-1` checksums remain unchanged until
the next release is explicitly published.

The corrected archive set was published as the prerelease
[llvm-23.1.2-actus-2](https://github.com/actus-lang/actus/releases/tag/llvm-23.1.2-actus-2)
by workflow [#42433465998](https://github.com/actus-lang/actus/actions/runs/42433465998).
The release assets were downloaded and checked against their accompanying
`.sha256` files before updating the contributor manifest:

- `x86_64-unknown-linux-gnu`: **161,765,900 bytes**, SHA-256
  `3dfe69ed3a2f1cb86e65c417ead3552d6dae6350b70947c3c35ff47cc0adfddc`.
- `aarch64-apple-darwin`: **104,259,476 bytes**, SHA-256
  `429f49aca94a89b39bf3c9491b90911cc3901206cfceb4fcbf5f4a9e3cf9cb53`.
- `x86_64-pc-windows-msvc`: **252,625,655 bytes**, SHA-256
  `919730c11401b7598fc8ee2b86eeb320e946d282a2d07a9ba972c25e18d594e3`.

The original diagnosis remains recorded: `actus-1` contained 2,653 Windows
ZIP entries with `compress_type=0` (`ZIP_STORED`), which explained why its
compressed size was almost identical to the extracted payload. `actus-2`
contains the deterministic DEFLATE level-9 output and retains the ZIP format.
The `actus-1` release remains immutable and untouched.

### 51.4 — Compiler identity and backend integration

T4 embeds and reports the LLVM identity. With all LLVM CI jobs now using the
verified T3 archive, strict LLVM builds reject an explicit unverified system
override with E1806. Cranelift-only builds and non-strict builds retain their
previous behavior. `release.yml` is intentionally unchanged: its release build
does not use `--strict`, and its migration to the bundled toolchain remains a
later T7 decision.

- [x] Add LLVM version, archive checksum, build revision, target, profile, and
      linker identity to `actus --version`.
- [x] Embed the deterministic identity in the build and verify it through the
      contributor bootstrap jobs ([PR #113](https://github.com/actus-lang/actus/pull/113)).
- [ ] Include the same identity in stack-limit reports and future structured
      MCP responses.
- [x] Preserve LLVM identity invalidation: `build.rs:8-18,155-175` tracks the
      selected prefix manifest, the bundled release manifest when applicable,
      LLVM feature selection, and LLVM environment inputs. The regression test
      is `llvm_identity_inputs_are_tracked_by_build_script` in
      `tests/architecture_conformance.rs:175-204`.
- [x] Keep LLVM-specific types and discovery inside the LLVM backend boundary
      ([PR #113](https://github.com/actus-lang/actus/pull/113)).
- [x] Verify static-link output and record binary-size and link-time evidence
      in [CI run 42544855953](https://github.com/actus-lang/actus/actions/runs/42544855953).

  The measurements below are debug builds after an explicit package clean and
  relink. Release sizes and release link times belong to the T7 release
  workflow.

  | Platform | Dynamic dependencies | Debug size | Stripped/PDB size | Relink time |
  | --- | --- | ---: | ---: | ---: |
  | Linux x86-64 | `libstdc++.so.6`, `libm.so.6`, `libgcc_s.so.1`, `libc.so.6`, system loader; no `libLLVM` | 226,811,264 bytes | 74,049,248 bytes | 20,467 ms |
  | macOS arm64 | `/usr/lib/libc++.1.dylib`, `/usr/lib/libSystem.B.dylib`, `/usr/lib/libobjc.A.dylib`, Foundation framework, `/usr/lib/libiconv.2.dylib`; no `libLLVM` | 88,009,736 bytes | 83,620,752 bytes | 20,886 ms |
  | Windows x86-64 | Windows system DLLs plus `MSVCP140.dll`, `VCRUNTIME140.dll`, `VCRUNTIME140_1.dll`, and `api-ms-win-crt-*`; no LLVM DLL | 69,102,080 bytes | PDB 198,422,528 bytes | 21,446 ms |

  Linux's dynamic `libstdc++.so.6` dependency and Windows' VC runtime
  dependencies are known states; their distribution policy is deferred to T7.

The stack-limit report and MCP identity integration remain deferred to
ADR-0103 S3; this phase does not claim that work is complete.

T4 closed; remaining item deferred to ADR-0103 S3.

### 51.5 — Bundled lld target contract

- [x] Complete bundled lld and compiler-rt builtin link integration; helper
      stack claims remain deferred to ADR-0103 S3.
- [ ] Integrate compiler-rt helper stack claims with the ADR-0103 S3 helper
      registry (deferred to ADR-0103 S3).
- [x] Package a matching `ld.lld` executable at `lib/actus/bin/ld.lld` and
      record it in the identity manifest without shadowing system `PATH` tools.
- [x] Select bundled lld only for the primary `thumbv7em-none-eabihf` and
      secondary `thumbv7em-none-eabi` freestanding paths.
- [x] Keep hosted Linux on its documented C linker/libc contract and preserve
      explicit `ACTUS_LINKER` overrides.
- [x] Reject linker version or target mismatches with stable diagnostics.
- [x] Add object, link, and execution evidence for the freestanding path.

The bundled freestanding path is proven by the LLVM backend fixtures and the
QEMU test `objects::executes_a_bundled_thumb_image_under_qemu`. The fixture
builds the Actus object through the public CLI with a manifest target of
`thumbv7em-none-eabihf`, links it with the bundled `ld.lld` and compiler-rt
builtins, rejects VFP instructions, and executes the image on
`qemu-system-arm` with a 30-second timeout. The linked image contains the
64-bit division builtin (`__aeabi_uldivmod` or `__udivdi3`). System
`clang`, `arm-none-eabi-*`, and `rust-lld` are poisoned in the relevant CI
jobs, so these tests cannot silently use a host ARM toolchain.

This section is the T6 bundled-lld work. The identity and strict-build work
belongs to 51.4; stack-limit reports remain under ADR-0103 S3.

### 51.6 — Clean-environment and determinism evidence

- [x] Audit external-process boundaries before making LLVM test tooling
      self-contained. The compiler source does not invoke LLVM inspection
      tools or `clang`; `build.rs` invokes the selected `rustc`, Git metadata,
      and the explicitly selected `llvm-config`. Hosted linking remains the
      configured native linker boundary. After T6, the freestanding fixtures
      use the bundled LLVM linker and builtins; system `clang` and
      `arm-none-eabi-*` are no longer CI dependencies for these tests.
- [x] Resolve LLVM inspection tools from the verified `llvm-config` prefix,
      require exactly LLVM 23.1.2, and reject a prefix mismatch with
      `LLVM_SYS_231_PREFIX`. The resolver is shared by the LLVM backend
      integration modules and appends `.exe` on Windows.
- [x] Parse `.stack_sizes` directly from emitted objects with the `object`
      crate. The parser follows relocation records to function symbols,
      decodes ULEB128 sizes, and rejects missing relocations, truncated
      records, ULEB128 overflow, invalid relocation widths, and unknown
      symbols. Unit tests cover the empty, multi-record, overflow,
      truncation, and unknown-symbol cases. The x86-64 stack contract keeps a
      cross-check against `llvm-readobj --stack-sizes`.
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

T7 implementation is on `feat/user-release-archive`. CI evidence, archive
sizes, checksums, and the extracted-archive verification run are recorded
after the three-host pull-request workflow completes. The old release
installers were removed: they downloaded or unpacked LLVM separately and did
not preserve the relocatable user layout, identity manifest, runtime archive,
or verified bundled linker contract.

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

