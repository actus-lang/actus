# ADR-0048: Explicit Limitless Source Scopes and Conformance Exceptions

- Status: Proposed
- Date: 2026-09-30
- Scope: Actus metadata, source conformance limits, project configuration,
  diagnostics, and strict validation policy

## Context

Actus uses source-size and function-size limits as architectural guardrails.
The preferred thresholds encourage decomposition before a module becomes
monolithic, while the hard thresholds protect compiler reviewability,
diagnostic quality, and long-term maintenance.

Those limits are defaults for responsible production code, not a claim that
every valid program can be decomposed without cost. Generated compatibility
surfaces, deliberately cohesive protocol tables, low-level declarations, and
other narrowly justified boundaries may need to exceed the preferred limits.
Forcing every such declaration into artificial helper files can make the
architecture less clear instead of improving it.

The previous suppression shape was not an Actus language feature. A source
comment such as `actus: allow(source-limit)` could be mistaken for an
ordinary comment, was difficult to scope precisely, and provided no
structured reason or ownership information. Source-local suppression is also
incompatible with deterministic strict conformance because a comment can
silently disable an architectural policy.

Actus therefore needs an explicit, parseable exception mechanism that keeps
the default limits intact, makes the scope visible at the declaration or file
boundary, and allows projects to assume responsibility for intentional
exceptions. The mechanism must work consistently in the parser, semantic
validation, conformance scanner, CLI diagnostics, LSP diagnostics, and
`Actus.toml` configuration.

The source-size metric measures code-bearing lines rather than raw physical
lines. Blank lines, Rust-style `//` and `/* ... */` comments, and Actus `#`
comments are excluded. An inline comment does not create an additional measured
line when code is already present. Actus triple-quoted `""" ... """` blocks
remain counted because they are language constructs and part of the
documentation contract. This allows production sources to use concise inline
notes without weakening the architectural limits on executable code.

## Decision

Actus introduces one metadata directive with an explicit scope argument:

```actus
meta limitless("verb")
verb legacy_adapter() {
    ...
}
```

The directive immediately preceding a verb applies only to that verb. It
removes the preferred and hard function-size diagnostics for the annotated
verb, but it does not remove the file-size limit from the containing source
file.

At the beginning of a source file, the file-scoped form applies to the whole
file:

```actus
meta limitless("file")

"""
The file intentionally keeps one cohesive compatibility boundary.
"""
open verb compatibility_surface() {
    ...
}
```

`meta limitless("file")` removes both the file-size diagnostics and the
function-size diagnostics for declarations in that file. It is a file-level
decision and must appear before declarations; placing it after a declaration
does not retroactively change the file policy.

### Accepted scopes and placement

The initial scope registry contains exactly two values:

- `"verb"`: valid only immediately before one verb declaration;
- `"file"`: valid only in the file metadata prelude before declarations.

Unknown scope strings, duplicate directives, directives separated from their
target declaration, and directives placed inside a verb body are rejected as
deterministic metadata diagnostics. A verb directive cannot affect a sibling
verb, an imported module, or an entire file.

The directive is metadata, not a comment convention. It is preserved by the
formatter, represented in the AST, exposed through LSP document symbols and
diagnostics, and included in source identity wherever conformance policy is
part of the artifact contract.

### Project configuration

When the whole project intentionally accepts responsibility for source size,
the package may select the limitless source policy directly in its existing
`[package]` configuration:

```toml
[package]
name = "actus"
version = "0.1.0"
edition = "alpha"
entry = "main"
source_limits = "limitless"
```

`package.source_limits` accepts only `"limitless"`. When the field is absent,
the architectural limits remain enabled implicitly; projects do not need to
spell out a `default` value. `"limitless"` applies to every source file and
every verb owned by the project, including sources discovered through the
project's configured source root. It does not alter dependencies, parser limits,
recursion limits, memory bounds, ownership checks, unsafe-bridge checks,
documentation requirements, or runtime safety contracts.

Removing the limits is not the recommended project configuration. The default
limits are an architectural guardrail and should remain enabled for normal
production packages. The project-wide setting is nevertheless an explicit
freedom-of-choice switch: a team that accepts responsibility for maintaining
large cohesive sources may select `"limitless"` once, without repeating one
table entry for every file. File- and verb-level
metadata remains available when a project wants to keep the default policy
globally and exempt only a narrowly defined boundary. The compiler rejects
unknown values, duplicate `package.source_limits` declarations, and attempts
to use the project switch to modify unrelated safety policies.

The selected policy is part of configuration identity and is reported in
structured conformance metadata. It is therefore visible in review and
reproducible across machines without requiring per-file ownership records.

### Strict conformance policy

Without an explicit limitless directive or project-level limitless policy, the
normal preferred and hard limits remain enforced exactly as before.

In non-strict commands, an accepted limitless scope is visible in structured
conformance metadata and may produce a concise review note. It never hides
syntax, semantic, ownership, code-generation, or runtime diagnostics.

In `--strict` commands, source-local metadata is accepted when the project
policy permits it. A project-wide `source_limits = "limitless"` selection is
itself the explicit approval for all project sources. A repository or CI
policy may still require the default policy for production packages; that
policy must fail closed with a stable diagnostic rather than silently changing
`package.source_limits`. The default strict policy must never treat an
unrecognized comment or arbitrary text as an approval.

Limitless applies only to the selected conformance dimensions. It does not
disable parser limits, recursion limits, memory bounds, ownership checks,
unsafe-bridge checks, documentation requirements, or any runtime safety
contract. A limitless file also remains subject to the repository's file-size
review and source exception inventory so that the opt-out is visible to
maintainers.

### Diagnostic and tooling contract

Diagnostics for invalid metadata are represented independently from terminal
rendering and are available through CLI and LSP adapters. They identify the
directive span, requested scope, target source or verb, and the corrective
placement or project-policy requirement.

The conformance model records for every accepted exception:

- canonical source path;
- scope (`file` or `verb`);
- target verb name when applicable;
- declaration or file source span;
- whether the approval came from source metadata or `Actus.toml`;
- the active project policy (limits enabled or `limitless`);
- the approval origin (source metadata or project configuration).

The formatter must preserve directive placement and its string argument.
LSP code actions may suggest a project policy change, but must not insert one
without an explicit user action because it is a policy decision.

## Invariants

1. Architectural limits remain enabled by default.
2. `meta limitless("verb")` affects exactly one immediately following verb.
3. `meta limitless("file")` affects exactly one source file and all of its
   function-size checks.
4. No comment, unknown metadata, wildcard, or malformed project policy can
   disable a conformance limit.
5. Limitless exceptions never disable semantic, ownership, safety, ABI, or
   runtime validation.
6. The absent `package.source_limits` field keeps limits enabled, while the
   only accepted explicit value is `"limitless"`.
7. Project policy and source metadata cannot silently conflict or redefine
   unrelated safety policies.
8. CLI and LSP consumers receive the same structured exception model and
   stable diagnostics.
9. Formatting is idempotent and preserves the metadata boundary.
10. Strict builds fail closed when the active repository policy requires the
    default source-limit policy and the project selects `limitless`.

## Consequences

Positive consequences:

- Valid cohesive or generated source can opt out without artificial
  decomposition.
- The scope is visible in the language instead of hidden in comments.
- The project can accept responsibility once in `Actus.toml` without
  repeating configuration for every source file.
- The compiler can distinguish policy exceptions from ordinary source text.
- Future scopes can be added to one explicit metadata registry without
  inventing separate directives for every declaration category.

Costs and risks:

- The parser, AST, semantic policy, conformance scanner, formatter, LSP, and
  project configuration all need coordinated support.
- A project-wide limitless policy can become technical debt if it is not
  reviewed as part of the package's conformance policy.
- Strict CI policy must be made visible so local acceptance and CI rejection
  are not surprising.
- Source metadata and the project policy become part of the reproducibility
  and cache identity of conformance analysis.

## Non-goals

- This ADR does not remove the default source or function limits.
- This ADR does not permit arbitrary compiler or runtime resource exhaustion.
- This ADR does not introduce per-dependency or workspace-wide exemptions;
  the project-wide setting applies only to the configured project sources.
- This ADR does not define generated-code exemptions outside the explicit
  source and project scopes.
- This ADR does not change the Actus ownership model or native ABI.
