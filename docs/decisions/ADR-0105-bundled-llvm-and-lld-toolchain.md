# ADR-0105: Bundled LLVM and lld toolchain

## Status

Accepted.

## Date

2026-11-05.

## Accepted

2026-11-09 (implemented across the Phase 51 PRs listed in the roadmap).

The decision was implemented in PRs #113–#119.

## Scope

Compiler distribution, LLVM discovery, lld integration, contributor
bootstrap, release archives, toolchain identity, and self-contained CI.

This ADR defines the target architecture and records the implementation delivered
through the Phase 51 roadmap and PRs #113–#119.

## Context

ADR-0098 makes LLVM the primary native backend. The current development and CI
workflow still expects an LLVM installation to be present on the host and
discovers it through `llvm-config` or `LLVM_SYS_231_PREFIX`. That is suitable
for backend development, but it is not a complete product distribution
contract: a user who installs Actus should not have to install a matching LLVM
development package, headers, linker tools, and C++ runtime independently.

The problem is not only convenience. An unpinned host installation can change
LLVM IR lowering, object layout, diagnostics, stack-size evidence, linker
behavior, and generated artifacts. A different LLVM patch release may also
produce a different result while satisfying the same major-version lookup.
The compiler therefore needs one reproducible toolchain identity for local
bootstrap, CI, and release artifacts.

The bundled toolchain must preserve Actus's existing contracts:

- source semantics, ownership, cleanup, and serialized formats remain
  unchanged;
- LLVM remains behind the LLVM backend boundary described by ADR-0098;
- machine-level stack evidence keeps the exact proof identity required by
  ADR-0103;
- integer-only and deterministic paths do not gain hidden floating-point,
  allocation, or filesystem behavior; and
- a missing or incompatible toolchain fails with an explicit diagnostic,
  never with a silent backend fallback.

## Decision

Actus will distribute and bootstrap a fixed LLVM 23.1.x toolchain, including
the lld component required by the supported link paths. The exact patch level
for the first release is **LLVM 23.1.2**. Every archive, build job, and
reported toolchain identity must use that patch level unless this ADR is
amended.

The T2 Linux source build pins LLVM to commit
`85ac560262434c9ccfc0c183ec22d4138ed647fb`, the dereferenced commit for the
`llvmorg-23.1.2` release tag. A tag name alone is not sufficient provenance.

The first supported architecture includes X86, ARM, and AArch64 LLVM targets.
The first freestanding targets are `thumbv7em-none-eabihf` as the primary
Cortex-M4F target and `thumbv7em-none-eabi` as the secondary soft-float/no-FPU
target. The eabihf choice matches the ABI used by Cortex-M4F devices such as
STM32F411 and compatible vendor C libraries. The toolchain build must not
include unrelated targets or optional external dependencies merely because the
host distribution provides them.

The implementation is divided into independently reviewable steps:

1. build and publish reproducible, checksum-addressed toolchain archives;
2. provide contributor bootstrap scripts that select those archives;
3. make the compiler and backend consume the selected prefix without requiring
   a system LLVM installation;
4. integrate bundled lld for the freestanding link contract; and
5. add clean-environment, identity, packaging, and release acceptance tests.

No step may weaken the existing explicit backend, target, or linker
selection rules.

## LLVM linkage model

The default release model is a statically linked LLVM backend: the Actus
compiler links the required LLVM libraries into the compiler binary, while
the bundled development archive remains available for `llvm-config`, headers,
and build-time tooling. This avoids a runtime dependency on a host-installed
`libLLVM`, makes the compiler artifact self-contained, and prevents loader
search paths from selecting an ABI-incompatible shared library.

The release implementation must measure the static binary size and link time.
If the measured result violates the published release budget, a shared
`libLLVM` variant may be proposed in a separate ADR amendment. Such a variant
would require an explicit `$ORIGIN/../lib` (or platform-equivalent) loader
contract, signed library and executable artifacts, platform-specific
notarization review, and a test proving that an unrelated system library
cannot be selected accidentally. Shared linking is not an implicit fallback.

| Concern | Static LLVM in `actus` | Shared `libLLVM` |
| --- | --- | --- |
| Runtime installation | Self-contained compiler binary | Requires a matching packaged library |
| ABI risk | Resolved at build time | Loader can select an incompatible host library |
| Distribution | Larger executable, simpler runtime layout | Smaller executable, larger runtime contract |
| Reproducibility | Strong when archive and linker are pinned | Requires library identity and loader-path tests |
| Signing/notarization | Sign one main executable plus tools | Sign and notarize executable and shared libraries |
| Platform complexity | Mostly build-time complexity | Build-time and runtime loader complexity |

The static model is the accepted default for the first implementation. The
release process must still publish the development prefix because `llvm-sys`,
the shim build, and target-specific inspection tools need the matching headers
and utilities.

## Pinned LLVM build

The release builder must build LLVM 23.1.2 with a reproducible configuration:

- `LLVM_TARGETS_TO_BUILD=X86;ARM;AArch64`;
- `LLVM_BUILTIN_TARGETS=thumbv7em-none-eabihf;thumbv7em-none-eabi`, with the
  LLVM 23.1.2 `BaremetalARM.cmake` pattern: `CMAKE_SYSTEM_NAME=Generic`,
  `COMPILER_RT_BAREMETAL_BUILD=ON`, `COMPILER_RT_OS_DIR=baremetal`, and
  explicit Thumb/Cortex-M4 compiler and assembler flags for each ABI;
- zlib disabled;
- zstd disabled;
- libxml2 disabled;
- terminfo disabled;
- libedit, libpfm, CURL, httplib, FFI, Z3, libFuzzer, ORC, context profiling,
  GWP-ASan, CRT, assertions, and the LLVM shared library disabled;
- `LLVM_STATIC_LINK_CXX_STDLIB=ON`;
- `LLVM_DISTRIBUTION_COMPONENTS` plus `install-distribution` used so only the
  required LLVM headers/libraries, `llvm-config`, lld, compiler-rt builtins,
  and inspection tools are installed; Clang is built as a dependency but is
  not included in the development archive;
- compiler and linker flags are recorded in the archive manifest; and
- the C++ runtime policy is explicit for every supported host.

The Linux archive is built in a pinned image with glibc **2.28** as the
compatibility baseline. The release job must execute the resulting user
archive on that baseline and on the supported newer Linux images. The archive
must not accidentally acquire a newer glibc symbol requirement from the CI
runner.

The macOS development archive is built on the fixed `macos-15` arm64 runner
with Xcode 16.4, the macOS 13.0 deployment target, and the selected SDK
recorded in its manifest. It uses the system `libc++`; it does not ship a
private C++ runtime. The first macOS archive is
`aarch64-apple-darwin`. An x86_64 macOS archive is deferred until a reliable
cross-build or runner contract is approved; it is not silently substituted by
an arm64 archive.

On Windows, the LLVM CMake build must use
`CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL`, matching Rust's `/MD` runtime
contract. A mismatch between LLVM's C++ runtime and Rust's runtime is a build
failure, not a packaging variation.

The Windows development archive uses the dynamic MSVC runtime and must record
the selected MSVC toolset. The later user archive must make an explicit
decision about delivering the VC runtime, either through the redistributable
installer contract or app-local DLLs; the development archive does not settle
that release decision.

The first Windows contributor archive is built on the fixed `windows-2022`
runner with the pinned MSVC 14.44.35207 toolset, Ninja, and `sccache`. It is a
`.zip` archive using deterministic DEFLATE compression at level 9, and its
executables use the `.exe` suffix. Verification uses
`dumpbin /dependents` and rejects dependencies outside the Windows system DLL
set and the `/MD` VC runtime family. The archive manifest records the toolset
identity and the `MultiThreadedDLL` CMake value.

The development archives retain only the linker names used by Actus: `ld.lld`
and `lld-link`, plus the canonical `lib/actus/bin/ld.lld` copy. The unused
`ld64.lld` and `wasm-ld` aliases are not distributed. The standalone legacy
`libLTO` artifact is also excluded because no Actus build, `llvm-sys` path, or
test consumes it; the LLVM LTO libraries required by other LLVM components are
not inferred to be removable from the distribution component as a whole.

The freestanding toolchain must also provide the compiler-rt builtins needed
by each supported freestanding target. This includes `__aeabi_uldivmod` and
every other compiler-generated helper required by the emitted object. The
archives are published at:

```text
lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a
lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a
```

The preferred v1 decision is to package the matching compiler-rt builtins
archives and link them explicitly. An alternative runtime implementation is
acceptable only when the final link command and symbol manifest prove that
every required helper is supplied.

Although `thumbv7em-none-eabihf` permits hardware floating-point instructions,
it does not relax Actus's zero-float contract. T2 verification must disassemble
an integer-only Thumb fixture and reject VFP instructions (`vadd`, `vmov`,
`vldr`, `vstr`, `vpush`, and equivalent operations). The eabihf ABI and the
object-level zero-float policy are separate contracts.

Compiler-generated helpers must be registered at the runtime/backend boundary
described by ADR-0103. A builtin is not an untracked linker convenience: its
symbol, archive identity, target, ABI, and stack claim must be present in the
helper registry used by stack-limit closure. A missing registry entry or a
helper supplied by an unverified archive is a strict-build error.

The Linux release builder statically links libstdc++ without making libc
static. Because llvm-sys emits an explicit `-lstdc++`, the driver's
`-static-libstdc++` option does not override that later explicit library
selection. The build therefore creates a temporary `libstdc++.so` symlink to
the toolchain's `libstdc++.a` and passes that directory through
`LLVM_SYS_SYSTEM_LIBRARY_DIRS`; the linker then resolves the explicit
`-lstdc++` to the archive. The extracted release verifier's `ldd` output is
the authority for this contract and must contain no `libstdc++.so.6`. A fully
static libc is not required and must not be implied; the manylinux glibc 2.28
baseline remains the compatibility boundary.

The preferred release path builds archives in CI from a pinned source
revision and publishes them as release assets. A transitional fallback may
use the official LLVM 23.1.2 release archives when building from source would
exceed the CI runner's time or storage budget; this does not remove checksum
verification.

T2 development archives are packaged with the repository's deterministic
Python packager. macOS uses `.tar.xz`; Windows uses `.zip`. File ordering,
archive timestamps, ownership metadata, and permissions are normalized where
the format permits. Linux retains its already-published archive identity and
does not migrate to the new packager in T2.

Each published archive has a repository-tracked SHA-256 entry. The checksum
covers the complete archive, not only the LLVM source or one binary. The build
must fail if the downloaded archive, extracted manifest, LLVM version, target
set, builtins archive, or runtime helper registry does not match the
repository declaration.

## Toolchain identity

Every compiler build and every machine-level proof report must be able to
identify the toolchain with at least:

- LLVM version and patch level;
- archive SHA-256;
- Actus target triple;
- compiler profile and backend;
- toolchain build revision; and
- relevant linker identity; and
- the immutable build-image digest for source-built archives.

The identity is read from the selected `toolchain.json` at build time and is
embedded in the compiler binary and generated proof metadata. It is not
reconstructed later from whatever `llvm-config` happens to be on `PATH`.

`actus --version` must print the selected LLVM identity when LLVM is available.
The stack-limit report required by ADR-0103 must include the same identity so
that a frame claim cannot be reused after an LLVM archive change. Future MCP
responses must expose the identity through a structured field rather than
requiring consumers to parse human-readable text.

The identity is part of generated artifact compatibility. An object or stack
claim produced with a different LLVM archive, target, profile, or backend is
not silently reused.

## Contributor bootstrap

The repository will provide equivalent bootstrap entry points:

- `scripts/bootstrap_llvm.sh` for Unix-like hosts; and
- `scripts/bootstrap_llvm.ps1` for Windows.

Both scripts download the declared archive into:

```text
.actus-toolchain/llvm-23.1.2/
```

The scripts must:

1. select the archive by host and architecture from a checked-in manifest;
2. download over HTTPS;
3. verify the complete archive SHA-256 before extraction;
4. extract into a versioned directory without modifying an existing verified
   directory;
5. validate `llvm-config`, the LLVM version, target list, and required lld
   tools; and
6. configure Cargo discovery through `.cargo/config.toml` using
   `LLVM_SYS_231_PREFIX` with `relative = true`.

System LLVM remains available only through an explicit override such as a
developer-provided environment variable. The compiler and scripts must emit a
warning when that override is used, including the discovered version and
prefix, and mark the resulting identity as `unverified`. An unverified
identity is permitted for ordinary exploratory development only. A `--strict`
build rejects it, including a `stack_limit` proof, because the proof identity
cannot be tied to a published archive checksum. The diagnostic must identify
the override and explain how to bootstrap the verified toolchain.

The bootstrap scripts must be idempotent and must not modify a user's shell
profile, global Cargo configuration, or system package manager state without
explicit user action.

## lld integration

The first release will package lld as a separate tool, following the
`rust-lld`-style boundary, rather than calling the unstable internal C++
`lld::lldMain` API from the Actus process. A separate executable gives the
linker its own versioned boundary, simplifies archive layout and signing, and
avoids coupling Actus to lld's internal C++ ABI.

For v1, the bundled lld default is limited to the freestanding
`thumbv7em-none-eabihf` and secondary `thumbv7em-none-eabi` link contracts.
Hosted Linux continues to use the
configured system C linker and libc startup objects; hosted Windows and Apple
link contracts remain explicit and target-specific. `ACTUS_LINKER` remains an
explicit override and is never replaced silently.

Freestanding linker selection is ordered as `ACTUS_LINKER`, manifest
`[build].linker`, then the bundled linker. The bundled lookup first checks
`<current_exe>/../lib/actus/bin/ld.lld` when
`<current_exe>/../share/actus/toolchain.json` exists, then the build-time LLVM
prefix. The selected executable must report `LLD 23.1.2`. `ACTUS_LINKER`
applies to every target, so a cross-target build from a hosted host to a
freestanding target must not set it. A target-bound override is deferred to a
future decision.

The freestanding link path must select the bundled lld executable by identity,
pass the target and linker script explicitly, and reject a mismatched linker.
The link step must report the selected linker and checksum in verbose build
metadata. Library-based lld integration may be reconsidered only after the
separate executable boundary has complete release and cross-platform evidence.

All freestanding Actus functions, including external declarations, are emitted
as `nounwind`. A native provider must not throw an exception across the
freestanding boundary; doing so is undefined behavior under the native-boundary
contract in ADR-0103. ARM `.ARM.exidx` entries remain valid `.cantunwind`
records, and the recommended freestanding linker script discards `.ARM.exidx*`
and `.ARM.extab*` after linking.

Compiler-rt's registered freestanding builtins are selected independently of
linker selection from the `compiler_rt_builtins` entries in
`share/actus/toolchain.json`. In particular, the registry supplies the
`__aeabi_memcpy`, `__aeabi_memmove`, `__aeabi_memset`, and `__aeabi_memclr`
implementations; the linked aggregate fixture has no undefined symbols under
`llvm-nm -u`, so Actus does not need a separate freestanding memcpy runtime.

The executable is intentionally not placed in the top-level `bin/` directory,
where it could shadow a system `lld` on `PATH`. Its canonical location is
`lib/actus/bin/ld.lld`; Actus resolves that path from its own installation
prefix and passes the absolute path to the freestanding link step.

## Distribution layout

Release artifacts use target-specific archives:

```text
actus-<version>-<host-triple>.tar.xz
actus-<version>-<host-triple>.zip
llvm-dev-23.1.2-<host-triple>.tar.xz
llvm-dev-23.1.2-<host-triple>.zip
```

The exact extension is selected by the host platform. These are two different
artifacts with different audiences:

- `actus-<version>-<host-triple>` is the user distribution. It contains the
  compiler, standard library/runtime support, supported target link tools,
  identity metadata, licenses, and the compiler-rt builtins required by every
  supported freestanding target. Each target's archive is installed at
  `lib/actus/targets/<target>/libclang_rt.builtins.a`. It contains no LLVM
  headers, `llvm-config`, or other development-prefix files.
- `llvm-dev-23.1.2-<host-triple>` is the contributor/bootstrap archive. It
  contains the pinned headers, `llvm-config`, libraries, target tools,
  compiler-rt builtins, lld, manifest, and licenses needed to build Actus.

The user archive layout is:

```text
bin/actus
lib/actus/bin/ld.lld       # where the target contract supports bundled lld
lib/actus/                 # standard library, runtime support, and metadata
lib/actus/targets/<target>/libclang_rt.builtins.a
share/actus/docs/          # installed user-facing documentation
share/actus/toolchain.json # version, target, checksum, and build identity
licenses/                  # Actus and third-party notices
```

The contributor archive uses the same installation prefix for development
files and additionally contains `include/`, `lib/llvm/`, `bin/llvm-config`,
target inspection tools, and target-specific compiler-rt builtin archives.

The archive manifest must identify every executable and library that is
shipped. Checksums are published beside the release assets and reproduced in
the release manifest. Installation must be relocatable and must not modify a
shell profile without explicit consent. Separate SHA-256 entries for `ld.lld`
and each builtins archive move to the T7 user archive; T6 currently verifies
the archive SHA-256 and the bundled `LLD 23.1.2` version only.

## Licensing and notices

LLVM and lld are distributed under Apache License 2.0 with LLVM Exceptions.
Release archives must include the applicable LLVM/lld license text and
third-party notices under `licenses/`. The release checklist must verify that
the notices match the exact source revision and enabled components.

The Actus distribution must clearly distinguish Actus-owned code, LLVM/lld
components, and other bundled dependencies. No dependency may be copied into
the archive without a license record and checksum.

## Self-contained test design

The acceptance suite must prove both development bootstrap and user
distribution behavior from clean environments:

- a clean checkout with no system LLVM finds only the verified repository
  toolchain;
- a mismatched or corrupted archive fails before extraction or compilation;
- `llvm-config --version`, `actus --version`, and the manifest agree on
  LLVM 23.1.2;
- the embedded build-time identity comes from the selected `toolchain.json`;
- an explicit system override is reported as `unverified` and is rejected by
  `--strict` before stack-limit proof acceptance;
- repeated bootstrap produces the same verified directory contents;
- an explicit system override is visible in diagnostics and is never confused
  with the bundled identity;
- the compiler can build and test a minimal Actus program without a separately
  installed LLVM development package;
- bundled lld is selected only for the supported freestanding target;
- hosted targets retain their documented native linker contracts;
- the resulting object, symbol, stack-report, zero-float, and ABI tests pass;
- release archives contain all required runtime, standard-library, license,
  and identity files; and
- a second run from the same archive produces byte-identical compiler and
  generated artifacts where the existing determinism contract requires it.

The tests must not depend on the developer's home directory, shell profile,
current working directory, or pre-existing global LLVM installation.

### User release archive policy (T7)

The user archive is a relocatable installation rooted at the directory that
contains `bin/actus`. It contains `bin/actus`, `lib/actus/bin/ld.lld`, both
`lib/actus/targets/thumbv7em-none-{eabi,eabihf}/libclang_rt.builtins.a`, the
complete `lib/actus/std/` package including its `Actus.toml` and sources,
`lib/actus/runtime/<host-triple>/libactus_runtime.a`,
`share/actus/toolchain.json`, Actus and LLVM third-party licenses, and a
README. It does not contain LLVM headers, `llvm-config`, or development LLVM
libraries. The installed runtime and standard-library resolvers use the
executable-relative `share/actus/toolchain.json` marker; a marked installation
never falls back to a build-time runtime archive.

Linux release binaries use the manylinux glibc 2.28 baseline and statically
link libstdc++ while retaining dynamic glibc. macOS alpha binaries are
unsigned and use the system Foundation/libobjc runtime; notarization is not a
release requirement. Windows archives ship exactly the x64
`VCRUNTIME140.dll`, `VCRUNTIME140_1.dll`, and `MSVCP140.dll` files from the
selected MSVC redistributable. Hosted Windows builds still require `lld-link`
and the Windows SDK, neither of which is in the user archive; this is a known
alpha limitation. Freestanding ARM builds on Windows are fully supported
because they use the bundled `ld.lld` and compiler-rt builtins. Release binaries
are stripped; Linux `.debug`, macOS
`.dSYM`, and Windows `.pdb` files are separate debug artifacts with checksums.
The manifest's `release_artifacts` contains exactly five records: Actus,
`ld.lld`, both builtins archives, and the runtime archive. Separate SHA-256
records for those files apply to the user archive; T7 does not alter the
contributor archive identity.

The archive verifier links its freestanding ARM fixture by invoking the
installed `ld.lld` directly because the Actus CLI has no startup-object
argument. It then runs the linked image on Linux QEMU. Removing the installed
linker and requiring the same ARM operation to fail with E1880 proves that
verification is using the install layout rather than a host fallback. The
former `install.sh` and `install.ps1` were removed because they downloaded or
unpacked LLVM outside this relocatable, identity-checked archive contract.

LLVM inspection tools used by tests are an explicit exception to the general
"no external tools" rule: they are accepted only from the extracted,
verified archive prefix. The shared test resolver runs that prefix's
`llvm-config`, requires version 23.1.2, compares its canonical `--prefix`
with `LLVM_SYS_231_PREFIX` when set, and constructs an absolute path for each
tool. A missing tool or a mismatched prefix is a hard test failure. Tests do
not add the archive's `bin` directory to `PATH`, and verification scripts use
absolute archive paths so a missing archive tool cannot silently fall back to
the host installation.

The initial `.stack_sizes` proof is implemented in the LLVM backend boundary,
not in integration-test parsing code. It reads relocation-backed records from
the object section, resolves the function symbol at each relocation addend,
and decodes the following ULEB128 frame size. The parser reports malformed
records explicitly; it does not truncate or guess. The broader ADR-0103 S3
call-graph and transitive max-plus proof remains a later step.

The self-contained test design deliberately does not add `iced-x86`,
`capstone`, or another disassembler crate. LLVM's pinned inspection tools are
already part of the verified contributor archive, and using them preserves
the target-specific disassembly semantics used by the existing contracts
without adding another native decoder, dependency, or architecture policy.

Linux clean-environment evidence is provided by the pinned glibc container.
The hosted macOS and Windows runners contain preinstalled LLVM tools, so their
clean-environment proof is a strict archive-prefix check: every inspection
tool must resolve from the extracted archive before it is executed. This is a
documented limitation of hosted runner verification, not permission to use a
system tool accidentally.

The Windows verification additionally checks both Thumb builtin archives for
`__aeabi_uldivmod`, validates the bundled `ld.lld.exe`, checks the `/MD`
dependency contract, and disassembles the integer-only Thumb fixture to reject
VFP instructions. The compiler-rt builtin build must use the freshly built
in-tree Clang tool, not `clang-cl`; a failed or ambiguous builtin build is a
hard CI failure rather than a fallback to a host LLVM installation.

## Alternatives considered

### Require system LLVM

Rejected as the default because it makes installation, CI, stack evidence,
and generated artifacts depend on an uncontrolled host environment. It remains
an explicit development override.

### Build LLVM from source for every local build

Rejected because it creates unacceptable build latency, network and toolchain
requirements, and version drift. Source builds remain a release-builder
implementation detail if needed to produce the pinned archives.

### Use the official LLVM 23.1.2 release archives as the permanent source

Not selected as the preferred release path because an official prebuilt
archive does not prove Actus's exact CMake configuration, target selection,
optional-dependency policy, compiler-rt builtin contents, or build environment.
It also weakens source-to-binary provenance compared with an Actus CI build
from a pinned LLVM source revision. It is nevertheless an explicit
transitional fallback when an Actus CI runner cannot complete the source build
within its resource budget. In that mode, the complete official archive is
still verified by the published SHA-256 and its provenance is recorded in
`toolchain.json`.

For v1, reproducibility means that the published archive checksum is the
identity used by bootstrap, builds, artifacts, and stack proofs. A bit-for-bit
rebuild of the official archive from source is not required. The loss is
limited to independent rebuildability and Actus-specific build provenance;
strict identity and checksum verification remain mandatory.

### Use shared libLLVM as the first release model

Rejected for the initial release because its loader, signing, notarization,
and runtime ABI contract is more complex than the static model. It can be
reconsidered with measured size evidence.

### Call lld through its internal C++ library API

Rejected for v1 because it couples Actus to lld's internal ABI and complicates
cross-platform packaging. A separate `ld.lld` process is a narrower and more
auditable boundary.

### Make Cranelift the distribution fallback

Rejected. ADR-0098 requires unavailable backend/target combinations to fail
explicitly. A bundled LLVM toolchain must not hide a packaging defect by
silently changing the native backend.

## Consequences

Positive consequences:

- normal Actus users do not need to install LLVM or lld separately;
- compiler, object, linker, and stack-proof behavior has one reproducible
  toolchain identity;
- CI can test a clean, product-like installation path;
- target-specific linker support remains explicit rather than accidental; and
- release artifacts carry their runtime and licensing evidence.

Costs and risks:

- release archives and compiler binaries become larger;
- the project must maintain archive builders, checksums, and platform assets;
- LLVM upgrades require a new identity, compatibility audit, and evidence;
- hosted and freestanding link contracts remain different; and
- static linking does not eliminate every platform-specific runtime or SDK
  requirement.

## Acceptance and follow-up

This ADR is accepted only after the Phase 51 gates provide reproducible
archives, bootstrap evidence, identity reporting, lld target evidence,
license validation, and clean-environment tests. Until then, the current
`llvm-config`/`LLVM_SYS_231_PREFIX` development contract remains active.

The following work is deliberately outside this ADR's implementation step:

- LLVM IR release optimization, tracked separately by Phase 50;
- new LLVM target support beyond X86, ARM, and AArch64;
- changing hosted linker policy;
- replacing the separate lld process with an in-process library shim; and
- changing Actus source semantics, ownership, ABI, or serialized formats.

