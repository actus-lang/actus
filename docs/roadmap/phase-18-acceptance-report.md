# Phase 18 Acceptance Report

## Scope

Phase 18 establishes strict Actus conformance as a fail-closed validation
contract for the compiler, standard library, runtime, test runner, and CI.
The release decision covers the implementation governed by
[ADR-0041](../decisions/ADR-0041-strict-actus-conformance.md).

## Accepted behavior

- `actus check --strict` validates source without emitting native artifacts.
- `actus build --strict` stops before code generation or linking when any
  source, semantic, architecture, documentation, target, or configuration
  validation fails.
- `actus test --strict` reports exact discovery and execution results and
  fails for compilation errors, non-zero processes, signals, and timeouts.
- Strict validation rejects unresolved declarations, ownership violations,
  facade and dependency violations, incomplete public documentation, and
  source-limit violations.
- Native runtime and C-ABI contracts preserve typed Actus errors and explicit
  ownership boundaries.
- Standard-library filesystem fixtures select host-native path storage without
  weakening the production path bridge.

## Evidence

The final local evidence was collected from a clean working tree on branch
`feat/phase-18.11-final-conformance-release`:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | passed |
| `cargo check --all-targets --all-features` | passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | passed |
| `cargo test --all-targets --all-features` | passed |
| `scripts/check_source_limits.sh` | passed |
| `git diff --check` | passed |
| `cargo run --bin actus -- lock --check` | passed |
| `scripts/check_stdlib.sh` | passed |
| `actus test --strict` | 37 passed, 0 failed |

Two strict builds of `examples/hello.act` produced byte-identical object
artifacts. Both runs produced SHA-256:

```text
44047dd8ea89b01f56b319016e178ac35c45789396d2a00e0e7c71c7864d05aa
```

The Linux, macOS, and Windows CI jobs also passed the strict quality matrix,
including standard-library conformance and native target checks.

## Release decision

Phase 18 satisfies the acceptance criteria of ADR-0041. Strict conformance is
accepted for release validation; non-strict mode remains an interactive
development workflow and is not release evidence. No undocumented source,
function, architecture, or runtime exception is accepted by this report.
