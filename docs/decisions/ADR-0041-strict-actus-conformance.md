# ADR-0041: Strict Actus Conformance and Fail-Closed Compilation

## Status

Accepted and implemented for Phase 18. Final evidence is recorded in the
[Phase 18 acceptance report](../roadmap/phase-18-acceptance-report.md).

## Context

Actus is intentionally explicit about ownership, module boundaries, runtime
contracts, and the distinction between accepted language behavior and
compiler implementation details. A permissive compiler can hide violations of
those rules by recovering from an unsupported construct, inserting an implicit
conversion, treating a warning as harmless, or emitting a partial program.

That behavior is unacceptable for the Actus compiler and standard library. A
successful check must mean that the source satisfies the language contract,
the ownership model, the module architecture, the public API documentation
rules, and the executable test contract.

The phrase “stricter than Rust” is a design intention, not a numerical
benchmark. This ADR defines measurable zero-tolerance behavior instead:
strict mode fails closed and accepts no unresolved correctness violation.

## Decision

Actus will provide a strict conformance mode based on four independently
enforced layers:

1. frontend and semantic completeness;
2. ownership, type, and exhaustiveness correctness;
3. module, documentation, and source-structure conformance;
4. build, runtime, and test execution conformance.

The strict mode will be exposed initially through:

```text
actus check --strict
actus build --strict
actus test --strict
```

The standard library and CI must use strict mode before Phase 18 is accepted.
After the final gate, strict mode becomes the default for standard-library
validation and release checks. A non-strict developer mode may remain for
interactive exploration, but it must never be accepted as release evidence.

### Conformance policy source and precedence

During Phase 18, `--strict` is the only supported command-line policy switch.
There is no `--no-strict` escape hatch and no `ACTUS_STRICT` environment
override. Environment variables that configure the toolchain, such as
`ACTUS_LINKER`, may affect linking but cannot change conformance mode.

Strictness is monotonic. The effective policy is the strongest requirement
from these sources, in descending authority:

1. target policy;
2. package policy;
3. workspace policy;
4. the explicit command-line `--strict` request.

For the initial implementation, only the command-line source can request
strict mode; target, package, and workspace files have no strictness field.
Their absence therefore means “no additional requirement”, never “disable an
explicit strict request”. Future policy fields may promote `Standard` to
`Strict`, but a lower-priority source must not downgrade a stronger policy.
Conflicting policy declarations are configuration errors and must be rejected
before source compilation.

## Fail-closed compilation contract

Strict mode has the following non-negotiable rules:

- Every warning is an error.
- Every unsupported construct is an error.
- Every unresolved name, type, role, target, module, or declaration is an
  error.
- No implicit conversion, ownership repair, default target, or inferred
  fallback may hide an error.
- No code generation or linking may occur after a frontend, semantic,
  architecture, or conformance error.
- Parser recovery may collect additional diagnostics, but recovered syntax may
  never be passed to code generation as if it were valid source.
- An empty declaration that requires members, or a placeholder implementation,
  may not be emitted as successful native code. Empty structs and method-less
  marker roles are valid only where their explicit zero-sized or marker
  semantics are defined below.
- A process that returns a non-zero test exit code is always a failed test.
- A missing, stale, malformed, or non-deterministic lockfile is an error.

The compiler may report multiple errors in one invocation, but the final exit
status is non-zero whenever any error exists. There is no source-level
suppression that converts a correctness error into success.

## Layer 1: frontend and semantic completeness

Strict validation must reject:

- unknown keywords, types, verbs, fields, modules, imports, and attributes;
- malformed metadata or metadata attached to an invalid declaration;
- unsupported syntax that was accepted only for parser recovery;
- declarations that are empty where their language contract requires members,
  incomplete, duplicated, or unreachable;
- implicit return paths that do not satisfy the declared return contract;
- unsupported generic applications or unresolved generic bounds;
- unknown target names or malformed target specifications;
- invalid primitive widths, numeric literals, and compile-time ranges;
- declarations that reach codegen without complete semantic validation.

All accepted syntax must have a defined AST, semantic rule, and codegen or
explicitly documented non-codegen meaning. Registration alone is not an
implementation.

### Declaration completeness classification

The compiler classifies top-level declarations before code generation rather
than treating every declaration as an executable definition:

- an `enum` must declare at least one variant. An empty enum has no constructible
  value or runtime pattern and is rejected with `E1810`;
- an empty `struct` is a valid zero-sized aggregate and remains available to
  type checking and layout computation;
- a method-less `role` is a valid marker contract, and an empty `perform` for
  that role is a valid explicit implementation of the marker contract;
- `unsafe extern` declarations describe imported ABI symbols and deliberately
  have no Actus body to lower;
- `import` declarations and module-facade `open` declarations control module
  visibility and deliberately do not lower to native functions;
- no separate placeholder declaration exists in the AST. A declaration is
  either validated under one of the contracts above or rejected before
  codegen.

These non-codegen meanings are explicit contracts, not code-generation
fallbacks. Any future declaration kind must add its AST, semantic rule,
code-generation behavior or non-codegen classification, and positive and
negative tests before it is accepted.

## Layer 2: ownership, type, and exhaustiveness correctness

Strict mode must enforce the complete Actus ownership model:

- `erg` is the active mutable owner;
- `abs` is a read-only non-owning view;
- `dat` is a terminal ownership transfer;
- `ins` is an exclusive call-scope loan;
- ownership state transitions must be explicit and reversible only where the
  language contract permits restoration;
- use-after-move, use-after-drop, double-drop, invalid mutation, invalid alias,
  and escaping borrow are errors;
- returned borrowed views must preserve their single origin and lifetime;
- C-ABI boundaries must validate role compatibility before lowering;
- `case` and `match` expressions must be exhaustive where their subject type
  requires exhaustiveness;
- primitive and enum patterns may not silently fall through to an implicit
  default;
- result and option branches must preserve payload ownership and cleanup;
- deterministic LIFO cleanup must be present on every normal and early-exit
  path.

Codegen must not infer or repair any of these rules. It receives only a
semantically valid program.

## Layer 3: module, documentation, and source structure

### Module architecture

Strict mode must reject:

- a directory module without its canonical module-named `.act` facade;
- external access to a sibling that is not exported by the facade;
- duplicate or ambiguous sibling declarations;
- imports that bypass the facade;
- reverse compiler-pipeline dependencies;
- parser, AST, semantic, and codegen responsibility violations;
- backend-specific types leaking into frontend declarations.

Generic implementation filenames are prohibited in Actus standard-library
modules unless a documented architectural exception is accepted. In
particular, the following names are forbidden by default:

```text
utils.act
helpers.act
common.act
misc.act
```

The same responsibility-based naming rule applies to Rust source files with
the repository's existing restrictions on `utils.rs`, `helpers.rs`,
`common.rs`, and `misc.rs`.

### Public documentation

Every public Actus declaration must have a complete `"""` block docstring.
This includes public structs, enums, fields, performances, verbs, unsafe
extern declarations, and facade exports where documentation is required by
the declaration kind.

Each public documentation block must state:

- the declaration's purpose and supported behavior;
- every parameter's ownership role and lifetime expectation;
- the return type and all meaningful `Result` or `Option` branches;
- possible typed errors and rejected inputs;
- allocation, mutation, cleanup, and ABI behavior where applicable;
- platform or target restrictions where applicable.

A one-line name restatement, an empty block, or a promise of behavior not
implemented by the runtime does not satisfy the rule. Rust public APIs use
Rust `///` documentation; Actus `.act` APIs use the canonical block-docstring
form above.

### Source limits

Strict validation must enforce the repository limits as errors:

- files above 300 lines require decomposition planning;
- files at or above the 400-line warning threshold fail strict validation;
- files above the 500-line hard limit are always rejected;
- functions above 30 lines require decomposition planning;
- functions at or above the 40-line warning threshold fail strict validation;
- functions above the 60-line hard limit are always rejected.

Generated files and explicitly identified tabular fixtures may be exempt only
through a repository-level validator rule. A local comment cannot suppress a
source-limit violation.

## Layer 4: build, runtime, and test conformance

Strict validation must verify the complete execution boundary:

- public runtime bridges have explicit status contracts;
- raw C/POSIX/Windows statuses do not leak through public Actus APIs;
- generated artifacts are deterministic and target-specific;
- lockfiles match manifests and dependency content;
- test fixtures are located under `tests/` and Actus library fixtures under
  `tests/library/`;
- Rust test logic must not be placed inside `tests/library/`;
- every `meta test` has a valid signature, deterministic input, and observable
  exit status;
- positive and negative tests exist for every new public operation family;
- platform-specific tests use explicit target metadata;
- test subprocesses cannot hang on inherited interactive input;
- cleanup and temporary artifacts are removed after both success and failure.

The test runner must report the exact number of discovered, passed, failed,
and filtered tests. A filtered test must be explainable by target selection or
an explicit test filter; it must not disappear silently.

## Migration from permissive checks

Migration is staged and explicit:

1. run `actus check --strict` on the package and classify every failure by
   diagnostic phase;
2. fix frontend, semantic, ownership, architecture, documentation, and
   source-limit failures without adding suppression annotations;
3. run `actus build --strict` and verify that no artifact is produced after a
   failed validation;
4. run `actus test --strict` and require deterministic discovery, execution,
   cleanup, and exit-status evidence;
5. enable the same strict commands in CI and standard-library checks;
6. reserve non-strict mode for local exploration only, never for release or
   conformance evidence.

The migration is complete only when the strict command set is reproducible
from a clean checkout and no workspace or environment setting can downgrade
the effective policy.

## Diagnostics contract

Strict diagnostics are compiler data, independent from terminal rendering.
Every diagnostic must contain:

- a stable `E####` code;
- severity;
- source path;
- half-open source span;
- a concise primary message;
- a rule-specific explanation;
- an actionable correction where one can be stated deterministically.

The existing frontend and semantic code mappings are inventoried in the
diagnostic renderer catalog. Phase 18 reserves `E1800`–`E1899` for strict-
conformance diagnostics. The planned categories and sub-ranges are:

| Category | Required coverage |
| --- | --- |
| strict frontend | unsupported, incomplete, or recovered declarations |
| strict semantic | unresolved types, roles, ownership, and exhaustiveness |
| strict architecture | facade, dependency, naming, and responsibility violations |
| strict documentation | missing or incomplete public contracts |
| strict limits | file and function size violations |
| strict execution | test, artifact, lockfile, and runtime contract violations |

The implementation reserves `E1800`–`E1809` for configuration, `E1810`–`E1819`
for frontend, `E1820`–`E1829` for semantic, `E1830`–`E1839` for architecture,
`E1840`–`E1849` for documentation, `E1850`–`E1859` for limits, and
`E1860`–`E1899` for execution and future strict expansion.

Diagnostics must be deterministic in ordering and content. Terminal colors,
LSP rendering, JSON output, and editor presentation are renderers only; they
must not change validation behavior.

The shared diagnostic catalog validator operates on registered code
definitions, rejecting duplicate stable codes and any attempt to associate one
code with contradictory severities. Repeated emitted instances of one code at
different source spans remain valid.

## Target and metadata rules

Target metadata is part of the language contract, not a comment convention.
Strict mode must:

- validate target names before filtering declarations;
- reject malformed or conflicting target metadata;
- reject multiple incompatible target declarations;
- filter non-matching declarations deterministically only after metadata is
  validated;
- ensure that target-specific tests and symbols cannot be referenced by an
  incompatible target;
- require explicit target evidence for platform-dependent behavior.

## Exceptions and generated material

Strict mode has no general-purpose `allow` escape hatch for correctness,
ownership, architecture, or documentation errors. Exceptions may exist only
for generated files, tabular fixtures, or a documented repository-level build
artifact rule. Every exception must identify its owner, scope, reason, and
validation replacement.

An exception must never weaken semantic analysis or permit invalid Actus code
to reach codegen.

## Non-goals

This ADR does not:

- redesign Actus ownership roles;
- change the lexer/parser/AST/semantic/codegen pipeline;
- introduce Phase 17 extension-platform implementation;
- define the Phase 18 roadmap gates;
- replace the existing diagnostics renderer or LSP protocol;
- promise that every warning from external tools can be mapped to an Actus
  language error without a defined contract.

## Implementation constraints

Implementation must follow this order:

1. inventory existing diagnostics, warnings, validators, and source-limit
   checks;
2. add accepted and rejected tests for each strict rule;
3. add the strict validation model and diagnostic codes;
4. connect strict validation to `actus check`, `build`, and `test`;
5. enable strict mode in standard-library and CI workflows;
6. verify deterministic diagnostics and exit codes on Unix, macOS, and
   Windows;
7. update the Phase 18 roadmap only after this ADR is accepted.

## Acceptance criteria

ADR-0041 is considered implemented only when:

- strict mode rejects every registered-but-unimplemented construct covered by
  the negative test matrix;
- no strict warning is emitted without a non-zero process result;
- no semantically invalid program reaches codegen;
- all public standard-library declarations satisfy documentation checks;
- all module, facade, naming, and source-limit checks are deterministic;
- native, cross-platform, and target-filtered tests pass;
- CI runs strict validation and remains green on Linux, macOS, and Windows;
- the final Phase 18 quality matrix is reproducible from a clean checkout.

## Consequences

Strict mode will reject more source than the default developer workflow and
will require additional diagnostics and negative fixtures. This is intentional:
the compiler must make incomplete work visible immediately rather than
silently accepting it. The tradeoff is higher implementation and migration
cost in exchange for predictable language behavior, safer runtime boundaries,
and reviewable standard-library quality.
