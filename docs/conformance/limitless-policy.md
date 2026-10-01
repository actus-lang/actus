# Limitless Source Policy

Actus source limits are enabled by default. The default policy is part of the
language's conformance contract: it keeps files and verbs reviewable, makes
responsibilities visible, and provides a deterministic signal before source
growth becomes architectural debt.

## Accepted exception forms

Use `meta limitless("verb")` only when one verb has a documented, cohesive
reason to exceed the normal limits. Use `meta limitless("file")` only when the
whole file is one deliberate boundary. The directive must be attached to the
intended declaration or file prelude; comments and formatter output are never
an approval mechanism.

The conformance report records every accepted scope with:

- the canonical source path;
- the scope (`file` or `verb`);
- the verb name when applicable;
- the source span;
- the approval origin (`SourceMetadata` or `PackageConfiguration`).

This record is the review evidence. A verb exception must not exempt sibling
verbs, and a file exception must not silently change ownership, semantic,
runtime, or ABI validation.

## Project-wide policy

`[package] source_limits = "limitless"` is an explicit package-level decision.
Omitting the field keeps source limits enabled; the enabled default is not
written redundantly to `Actus.lock`. A limitless package is accepted for teams
that own the resulting review burden, but it is not recommended for normal
production packages.

Before approving a project-wide exception, review the package's source layout,
the reason limits are unsuitable, and the plan for preserving module
boundaries. Strict CI must run the conformance command and retain its
diagnostic and accepted-scope counts. Changes to the package policy must also
refresh the lockfile and artifact identity, so a policy change cannot be hidden
behind an unchanged build cache.

The exception manifest under `docs/conformance/source-exceptions.toml` remains
for reviewed generated or tabular fixtures. It does not replace source
metadata and cannot be used to suppress ordinary application source limits.
