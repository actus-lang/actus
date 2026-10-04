# ADR-0058: Native Resolution of Facade-Exported Compile-Time Constants

- Status: Accepted and implemented / Phase 31
- Date: 2026-10-04
- Scope: compile-time constants, canonical module facades, semantic-to-native
  parity, generic specialization, object emission, and executable linking
- Depends on: ADR-0047 module-internal scope and facade visibility, ADR-0054
  package configuration and compile-time modules

## Context

Actus semantic analysis can resolve a public package constant that is exported
through a canonical parent facade and imported by a nested module. Native
lowering currently has a narrower view of the module graph. In that situation
the source is accepted, but native emission can report that the constant has no
native binding.

The failure is architectural rather than application-specific. A compile-time
constant must not become a runtime symbol merely because its declaration was
re-exported through more than one facade. Native lowering must receive the same
resolved constant environment that semantic analysis used, including constants
needed by generic specializations.

## Decision

The compiler will resolve compile-time constants from the canonical facade
dependency graph before native expression lowering. The resolved initializer is
then substituted as a typed compile-time expression in the consuming module;
the compiler does not duplicate the declaration, move it into the consumer, or
replace source code with a project-specific literal.

The implementation must preserve the distinction between:

- a public constant exported through a valid canonical facade chain;
- a private declaration visible only to its defining module and siblings;
- a direct child-module path that bypasses a parent facade;
- a runtime value or native external symbol.

Constants remain compile-time values. They do not acquire a native storage
allocation or an independently linkable ABI symbol unless a future, separate
decision explicitly introduces such a runtime representation.

## Resolution contract

1. The module aggregation layer constructs the public export graph using the
   existing canonical-facade and visibility rules.
2. For each native compilation unit, the compiler derives a deterministic map
   of constants reachable through that unit's legal imports and facade exports.
3. Only exported constants with a compatible declaration and initializer enter
   that map.
4. Constant substitution happens before native identifier lowering and before
   generic instances are emitted.
5. The substituted expression retains the declared Actus type and source span
   used for diagnostics.
6. A generic specialization receives the concrete constant environment of its
   instantiated module graph; it must not fall back to the unspecialized name.
7. Equivalent references to one constant produce no duplicate native symbol.
8. Conflicting declarations, incompatible types, private constants, and
   facade-bypass imports remain deterministic errors.

The resolver must use declaration identity and module provenance internally so
that two different constants with the same short name cannot silently shadow
one another. Public short-name lookup may remain the language-level contract,
but native lowering must never guess between multiple origins.

## Semantic and native parity

The same exported namespace must govern strict checking, test compilation,
object emission, executable emission, and language-server resolution wherever
the operation requires module visibility. Native lowering must not reimplement
a weaker, separate import rule.

The compiler must preserve existing ownership roles and constant types. The
following uses are all compile-time uses of the same resolved constant:

- scalar return;
- scalar initializer;
- predicate or branch condition;
- aggregate or pack field initializer;
- array capacity or const-generic argument where the type contract permits it;
- an expression inside a specialized generic verb.

## Determinism and ABI

Constants are ordered by canonical module identity and declaration identity
when environments are materialized. Native output must be stable across
repeated object and executable builds. Since constants are inlined typed
expressions, the fix must not add a runtime bridge, alter the external ABI, or
create duplicate native symbols.

If a constant initializer is not legal for compile-time evaluation, semantic
analysis must reject it before native code generation. Native lowering must not
silently evaluate runtime calls, ownership operations, or target-specific
behavior while resolving a constant.

## Required negative behavior

The implementation must continue to reject:

- a private constant reached from an external consumer;
- a direct child-module import that bypasses its canonical parent facade;
- an ambiguous or duplicate public constant export;
- a type-incompatible use of an exported constant;
- a generic specialization whose constant environment is incomplete or
  inconsistent.

These diagnostics must remain stable and must not be replaced by an internal
native-binding error.

## Required evidence

The implementation is accepted only with regression coverage for a neutral
package containing:

- a configuration facade exporting a constant from a sibling values module;
- a nested feature module importing the parent facade;
- a public verb returning the constant;
- scalar initializer, predicate, aggregate-field, and generic-specialization
  uses;
- strict checking, object emission, executable emission, and executable
  result verification;
- repeated-build comparison proving deterministic native output and no
  duplicate native symbol;
- rejected private-constant and facade-bypass cases.

The evidence must be collected at semantic, native, and executable boundaries.
Passing only `check --strict` does not close this decision.

## Consequences

Positive consequences:

- semantic and native compilation observe one visibility contract;
- package configuration constants remain centralized and reusable;
- generic nested modules can use typed constants without source workarounds;
- constants remain allocation-free and ABI-neutral;
- private declarations are not made public to satisfy the linker.

Costs and risks:

- module aggregation and native preparation need a shared constant-resolution
  boundary;
- generic specialization must carry compile-time environments explicitly;
- deterministic provenance and collision diagnostics require additional tests;
- object and executable paths must be kept in parity with the test runner.

## Non-goals

This ADR does not introduce runtime global variables, mutable configuration,
new import syntax, a second facade system, or application-specific constants.
It does not change any external project or embed AIE-specific behavior in the
Actus compiler.

## Implementation evidence

The implementation is contained in the native object-plan aggregation boundary.
Imported module objects now collect public declarations through the complete
transitive facade dependency graph, while preserving private visibility and
deduplicating declaration identities. Compile-time constants therefore reach
normalization and native lowering without acquiring runtime storage or ABI
symbols.

Evidence covers nested facade execution, generic specialization,
scalar/predicate/aggregate use, object and executable parity, private-constant
rejection, direct-facade-bypass rejection, deterministic object emission, and
symbol-table inspection.

The following checks passed after implementation:

```text
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
scripts/check_source_limits.sh
git diff --check
```

The implementation commits are `43b0819`, `2bcf9a4`, `ad17a60`, `90504ec`,
and `8b93b26` on the Phase 31 branch.
