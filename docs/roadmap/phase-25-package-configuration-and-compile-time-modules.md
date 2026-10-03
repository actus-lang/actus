# Phase 25: Package Configuration and Compile-Time Modules

Phase 25 defines a first-class, package-wide configuration boundary for Actus.
It builds on the hierarchical facade model from Phase 24 and gives packages a
safe place for typed compile-time policy values without introducing runtime
state or hidden imports.

The target shape is:

```text
src/
├── main.act
├── config/
│   ├── config.act       # reserved package configuration facade
│   ├── values.act       # public compile-time values
│   └── limits.act       # responsibility-specific values
└── aie/
    └── aie.act
```

Application and library modules may consume the public configuration facade:

```act
import config;

verb threshold() -> u8 {
    return DEFAULT_THRESHOLD;
}
```

The configuration subtree is intentionally one-way. Configuration may expose
typed compile-time declarations, but it must not import application, runtime,
hardware, standard-library, or domain modules. This prevents configuration
cycles and keeps package policy available before runtime code generation.

## Architectural contract

- `src/config/config.act` is the canonical package configuration facade when
  the package declares one.
- The `config` facade may open only configuration siblings or child facades.
- Configuration sources may contain compile-time constants and their required
  type-level declarations, but no runtime verbs, mutable state, or external
  imports.
- Other package modules may import `config`; configuration sources may not
  import other modules.
- Only declarations explicitly exposed through the configuration facade are
  public to package code.
- Configuration constants are compile-time values with no runtime storage,
  symbol, ABI entry, or emitted function.
- `Actus.toml` remains the authority for package, build, target, and runtime
  selection. Source configuration is not a replacement for manifest data.
- The semantic analyzer and native backend must consume one shared public
  configuration interface. They must not maintain separate visibility rules.
- Direct child-module imports, private constants, duplicate exports, type
  mismatches, import cycles, and runtime-dependent initializers remain rejected.

## Gate 25.0: Configuration contract and diagnostics

- [x] Define the reserved `config` module identity and its package-root scope.
- [x] Define the accepted filesystem layout and canonical `config/config.act`
      facade rule.
- [x] Define the allowed declaration categories inside configuration sources.
- [x] Define stable diagnostics for imports, runtime declarations, private
      exports, duplicate exports, cycles, and invalid configuration roots.
- [x] Record the distinction between source configuration and `Actus.toml`.
- [x] Add positive and negative contract fixtures before implementation.

Gate 25.0 is closed. ADR-0054 defines the reserved package-level `config`
facade, its one-way dependency contract, compile-time-only declaration
boundary, public visibility rules, diagnostic obligations, and separation from
`Actus.toml`. The positive and negative layouts in
`tests/fixtures/configuration/phase-25-contract.txt` establish the contract
before compiler implementation begins.

## Gate 25.1: Parser, resolver, and facade policy

- [x] Preserve existing `import` and `open` syntax without adding a second
      configuration language.
- [x] Resolve `config` through the package source root and canonical facade.
- [x] Allow configuration facades to open configuration siblings and nested
      configuration facades deterministically.
- [x] Reject `import` declarations from the configuration subtree.
- [x] Reject direct child imports that bypass `config/config.act`.
- [x] Reject ambiguous configuration file/directory shapes and missing
      canonical configuration facades.
- [x] Add resolver tests for sorted sources, overlays, and malformed trees.

Gate 25.1 is closed. The existing extensionless `import` and `open` syntax is
used unchanged. The resolver applies the canonical `config/config.act` facade
rule, preserves deterministic sibling/child discovery, and rejects missing or
ambiguous configuration roots. Module aggregation now rejects imports from
the entire `config` subtree with stable `E1113` diagnostics, while direct
configuration-child bypasses retain the stable `E1108` diagnostic. Module
tests cover public sibling resolution, nested configuration rejection, facade
bypass, canonical-root failures, sorted discovery, overlays, and malformed
facades.

## Gate 25.2: Semantic compile-time configuration

- [ ] Register public configuration constants in the shared module interface.
- [ ] Preserve constant type, initializer, source identity, and visibility.
- [ ] Validate configuration initializers as compile-time expressions only.
- [ ] Reject runtime calls, mutable bindings, ownership roles, and external
      dependencies inside configuration sources.
- [ ] Support transitive constant exports through nested configuration
      facades.
- [ ] Reject private constants, duplicate names, cycles, overflow, and type
      mismatches with deterministic diagnostics.
- [ ] Verify constants in initializers, `if` conditions, `case` guards,
      struct/pack literals, array capacities, and const-generic arguments.

## Gate 25.3: Const-only module and native lowering

- [ ] Permit a module whose public implementation contains no verbs.
- [ ] Prevent const-only modules from failing with “program has no verb
      declarations”.
- [ ] Represent public configuration values as compile-time bindings rather
      than runtime symbols or callable declarations.
- [ ] Inline transitive configuration constants before native lowering.
- [ ] Ensure nested constant expressions are fully substituted before codegen.
- [ ] Skip empty native object emission when a module has no runtime code, or
      define and test an equivalent deterministic metadata-only boundary.
- [ ] Verify executable and object builds without temporary anchor verbs.
- [ ] Confirm that no configuration constant introduces runtime storage or ABI
      identity.

## Gate 25.4: Package integration and dependency safety

- [ ] Make normal modules consume `import config;` through the public facade.
- [ ] Reject configuration-to-domain and configuration-to-runtime dependency
      cycles before semantic analysis or code generation.
- [ ] Preserve package dependency, runtime profile, target, and lockfile
      behavior.
- [ ] Ensure configuration values participate in cache/build identity through
      canonical source and interface fingerprints.
- [ ] Keep configuration boundaries deterministic across local dependencies,
      overlays, repeated loads, and filesystem ordering.
- [ ] Add accepted and rejected cross-module ownership/visibility fixtures to
      prove configuration remains compile-time-only.

## Gate 25.5: Tooling and developer workflow

- [ ] Make formatter preserve configuration facades, `open const` declarations,
      and deterministic sibling ordering.
- [ ] Make LSP diagnostics and navigation resolve configuration constants through
      the same public facade interface as native compilation.
- [ ] Keep malformed or partially edited configuration sources recoverable in
      the language server.
- [ ] Ensure the test runner treats configuration sources as package modules,
      not independent test roots.
- [ ] Document the configuration boundary in the Alpha guide and coding-agent
      guide without duplicating manifest configuration rules.
- [ ] Add production-shaped configuration examples and editor fixtures.

## Gate 25.6: End-to-end acceptance

- [ ] `actus check --strict` accepts a package with a const-only configuration
      subtree.
- [ ] `actus test --strict` executes tests consuming transitive configuration
      constants.
- [ ] Native object and executable builds succeed without an anchor verb.
- [ ] Constants work in initializers, conditionals, case guards, aggregate
      literals, array capacities, and const-generic arguments.
- [ ] Private constants, duplicate exports, direct child imports, cycles, and
      runtime-dependent initializers are rejected.
- [ ] Repeated builds and diagnostics are byte-for-byte/deterministically
      identical where the existing artifact contract requires it.
- [ ] Full Rust formatting, check, clippy, test, source-limit, documentation,
      architecture, and diff checks pass.

## Non-goals

This phase does not:

- replace `Actus.toml` package or deployment configuration;
- introduce mutable global variables or runtime dependency injection;
- add implicit imports or wildcard exports;
- make configuration a general-purpose runtime module;
- change ownership, borrowing, ABI, target, or standard-library semantics;
- add AIE-specific syntax to the Actus language.

## Completion criteria

Phase 25 is complete only when a package can expose typed, compile-time-only
configuration through a reserved global facade, while the compiler, native
backend, formatter, LSP, test runner, diagnostics, and cache identity all
honor the same visibility and dependency contract. The result must work
without anchor verbs or source-level workarounds and must include accepted and
rejected evidence for every public boundary.
