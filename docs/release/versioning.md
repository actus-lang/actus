# Release and Versioning Policy

Actus is currently in the `alpha` language edition. Until a stable edition is
declared, the compiler, language syntax, semantic rules, and internal unit ABI
may change only through the classification and migration rules below.

## Alpha edition and compiler commands

An Actus package declares `edition = "alpha"` in `Actus.toml`. `alpha` is the
only accepted language edition; an unknown edition is a configuration error.
The supported application workflow is:

| Command | Contract | Strict mode |
| --- | --- | --- |
| `actus check` | Parse, resolve, and semantically validate without code generation. | `--strict` supported |
| `actus build` | Emit an object or executable for the configured target. | `--strict` supported |
| `actus run` | Build a temporary executable, run it, and remove the artifact. | Normal run surface only |
| `actus test` | Discover and run Actus `meta test` verbs. | `--strict` supported |
| `actus fmt` | Format source or verify with `fmt --check`. | Not applicable |
| `actus watch` | Re-run the selected validation workflow after source changes. | Not applicable |
| `actus lock` | Validate or update deterministic dependency metadata. | Not applicable |

Release evidence uses the strict forms where the command supports them. The
absence of `--strict` from `run`, `fmt`, `watch`, and `lock` is intentional and
must not be hidden by documentation or shell wrappers.

## Change classification

- **Compatible:** preserves accepted source meaning, public signatures, error
  contracts, and artifact/lockfile interpretation.
- **Diagnostic-only:** changes wording, rendering, or source location while
  retaining the stable diagnostic code and acceptance behavior.
- **Edition-gated:** introduces syntax or semantics behind a new declared
  edition; `alpha` behavior remains unchanged.
- **Breaking:** changes accepted meaning, ownership, ABI, public library
  signatures, diagnostic code meaning, lockfile interpretation, or artifact
  identity. A breaking change requires an ADR or roadmap update and a
  migration note when practical.

Every change is classified in its commit/PR description. A feature is not
silently treated as compatible merely because old sources still parse.

## Stability contracts

Semantic diagnostics use stable `E####` codes within the Alpha edition. Message
wording and terminal rendering may improve without changing the code; a code's
meaning may change only through an edition-gated or breaking decision.

The public `std::io`, `std::fs`, and `std::path` APIs expose typed `Result` or
`Option` contracts. Raw runtime/C statuses remain implementation details of
unsafe bridges. Public API changes require a compatibility classification and
accepted and rejected tests.

`Actus.lock` is deterministic dependency metadata. Its package checksums and
source paths are part of the validation contract; stale or conflicting lock
state is rejected. Native artifacts are target- and profile-specific, and the
internal Actus unit ABI is unstable in Alpha, so artifacts require the exact
compiler and target contract that produced them.

## Experimental and deferred work

Experimental features must be identified in the relevant ADR and roadmap.
They are not Alpha guarantees until their acceptance tests, documentation,
and compatibility classification are complete. Deferred features must name a
future owner phase; extension-platform work remains outside the Alpha core
until Phase 19 handoff is complete.

The C ABI boundary remains the stable interoperability boundary for external
libraries and platform runtimes. The internal Actus unit ABI, compiler
implementation details, and unsupported host substitutions are not stable
interoperability contracts.

Alpha releases use `0.y.z` versions. Patch releases contain compatible fixes
and documentation corrections. Minor Alpha releases may add language features
or change explicitly unstable behavior only under the classification policy.
Release notes are generated from Conventional Commits and stored under
`docs/changelog/` for tagged versions.
