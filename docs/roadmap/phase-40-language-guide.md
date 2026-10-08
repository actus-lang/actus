# Phase 40: Actus Language Guide

## Purpose

Turn `ACTUS_CODING_AGENT_GUIDE.md` into a complete Actus language handbook for
people and coding agents.

The handbook must describe the language, its syntax, its ownership model, its
module system, its standard-library usage, its compiler workflow, and its
repository conventions in clear everyday language.

The handbook is a user guide. It is not a roadmap, an acceptance report, a
research note, a product claim, or a list of arguments for the language.

## Scope

This phase covers:

- the complete content of `ACTUS_CODING_AGENT_GUIDE.md`;
- the language concepts already documented in the repository;
- syntax and semantics represented by current source, examples, tests, and
  compiler diagnostics;
- standard-library usage instructions;
- compiler commands and project workflow instructions;
- examples that show how to write, organize, check, test, and build Actus code;
- instructions for both human readers and coding agents.

This phase does not introduce new language syntax, compiler behavior, standard
library APIs, runtime features, or repository policies.

## Handbook location

Create the handbook below `docs/guide/`.

Each directory is a subject category. Each Markdown file has one clear topic.
The root guide index is the entry point for readers and agents.

```text
docs/guide/
├── README.md
├── language/
│   ├── overview.md
│   ├── first-program.md
│   ├── source-files-and-layout.md
│   ├── declarations.md
│   ├── verbs-and-contracts.md
│   ├── expressions-and-statements.md
│   ├── types-and-literals.md
│   ├── structs.md
│   ├── packs.md
│   ├── enums.md
│   ├── arrays-and-buffers.md
│   ├── generics.md
│   ├── constants.md
│   ├── comments-and-documentation.md
│   └── formatting.md
├── ownership/
│   ├── overview.md
│   ├── roles.md
│   ├── erg.md
│   ├── abs.md
│   ├── dat.md
│   ├── ins.md
│   ├── moves-and-reuse.md
│   ├── borrowing-and-loans.md
│   ├── cleanup-and-scope.md
│   ├── case-and-branch-ownership.md
│   └── common-diagnostics.md
├── modules/
│   ├── overview.md
│   ├── files-and-directories.md
│   ├── canonical-facades.md
│   ├── public-and-private-symbols.md
│   ├── imports.md
│   ├── nested-modules.md
│   └── module-diagnostics.md
├── standard-library/
│   ├── overview.md
│   ├── runtime-profiles.md
│   ├── io.md
│   ├── filesystem.md
│   ├── paths.md
│   ├── strings-and-utf8.md
│   ├── time.md
│   ├── regions.md
│   ├── wire.md
│   └── errors-and-results.md
├── compiler/
│   ├── overview.md
│   ├── source-to-native-pipeline.md
│   ├── actus-toml.md
│   ├── lockfiles.md
│   ├── targets-and-profiles.md
│   ├── hosted-and-freestanding.md
│   ├── native-builds.md
│   ├── object-and-executable-output.md
│   ├── diagnostics.md
│   └── compiler-boundaries.md
├── workflow/
│   ├── project-setup.md
│   ├── check-test-and-build.md
│   ├── formatting.md
│   ├── examples.md
│   ├── benchmarks.md
│   ├── debugging.md
│   └── contribution-workflow.md
└── agent-reference/
    ├── reading-order.md
    ├── source-change-checklist.md
    ├── evidence-and-scope.md
    ├── repository-boundaries.md
    └── diagnostic-decision-tree.md
```

The exact file list may be adjusted during implementation when the source
inventory shows that a topic belongs in another category. Every moved topic
must remain represented in the handbook index.

## Handbook writing rules

- Explain each concept before showing its syntax.
- Use ordinary language before compiler terminology.
- Define every Actus-specific term at its first use.
- Show the smallest complete example before showing advanced variations.
- Explain what each line of an important example does.
- State ownership roles at declarations and call sites in examples.
- Describe inputs, outputs, ownership, cleanup, allocation, side effects, and
  failure behavior for public verbs and APIs where those details apply.
- Distinguish source syntax, compiler behavior, standard-library behavior, and
  repository workflow.
- Mark design-only or unavailable features as such when they appear in source
  material.
- Use stable terminology throughout all files.
- Link related topics instead of repeating long explanations.
- Keep examples consistent with the current Actus parser, semantic checker,
  native lowering, tests, and standard library.
- Use Actus `#` comments in Actus examples for short comments.
- Use triple-quoted documentation blocks only where Actus source requires
  declaration documentation.
- Keep the handbook free of arguments, marketing language, unsupported claims,
  and unrelated project-specific architecture.

## Information preservation rules

The migration from the current guide must preserve:

- every language rule;
- every syntax form;
- every ownership rule;
- every module and facade rule;
- every compiler command and option;
- every standard-library usage rule;
- every diagnostic explanation;
- every example that still describes current behavior;
- every limitation and boundary that affects code use;
- every link and cross-reference that remains relevant.

During migration:

1. Copy the source guide into a working inventory.
2. Assign every section to a handbook category and file.
3. Split sections by topic without deleting their meaning.
4. Rewrite only for clarity, ordering, duplication removal, and consistent
   terminology.
5. Preserve technical details when combining repeated sections.
6. Mark outdated syntax or behavior for correction instead of silently keeping
   it as current usage.
7. Record unresolved classification or wording questions in the phase notes.
8. Keep the original guide unchanged until the replacement handbook is
   complete and reviewed.

## Phase gates

### Gate 40.1: Source inventory

- [x] Read the complete `ACTUS_CODING_AGENT_GUIDE.md`.
- [x] Read the repository `AGENTS.md`, contribution rules, relevant ADRs, and
      all current roadmap files referenced by the guide.
- [x] Inspect the lexer, parser, AST, semantic analysis, ownership/cleanup,
      native lowering, standard library, tests, and examples named by the
      guide.
- [x] Create a section inventory with source headings, destination category,
      destination file, and migration status.
- [x] List duplicated, outdated, ambiguous, and cross-referenced sections for
      editorial treatment.

### Gate 40.2: Handbook information architecture

- [x] Create `docs/guide/README.md` as the handbook entry point.
- [x] Create the category directories.
- [x] Define the reading paths for beginners, experienced Actus developers, and
      coding agents.
- [x] Define shared terminology for roles, ownership, modules, facades,
      runtime profiles, targets, and native output.
- [x] Define the link and heading conventions used by all handbook files.
- [x] Add a source-to-destination map for the migrated guide sections.

### Gate 40.3: Language fundamentals

- [x] Write the language overview and mental model.
- [x] Write the first-program and project-layout guides.
- [x] Document source files, declarations, verbs, contracts, expressions,
      statements, types, literals, constants, comments, documentation, and
      formatting.
- [x] Document structs, packs, enums, arrays, buffers, and generics.
- [x] Add complete small examples for each fundamental subject.
- [x] Link all fundamental subjects from the handbook index.

### Gate 40.4: Ownership and cleanup

- [x] Document `erg`, `abs`, `dat`, and `ins` in separate focused files.
- [x] Explain ownership transfer, shared views, exclusive loans, moves, reuse,
      scope cleanup, and return cleanup.
- [x] Document ownership behavior through `case`, loops, conditional branches,
      nested calls, generics, structs, arrays, and enum payloads.
- [x] Add valid and invalid examples with the related diagnostic meaning.
- [x] Document explicit scalar reuse and indexed access where applicable.
- [x] Add an ownership diagnostic decision guide.

### Gate 40.5: Modules and facades

- [x] Document the relationship between files, directories, canonical facades,
      child modules, sibling modules, and parent exports.
- [x] Document public and private declarations.
- [x] Document import paths and facade-only access.
- [x] Document nested modules and their source layout.
- [x] Add examples for a single-file module, a directory module, a nested
      facade, and a public child module.
- [x] Add module-resolution and visibility diagnostics with remedies.

### Gate 40.6: Standard-library handbook

- [x] Create a standard-library overview and runtime-profile guide.
- [x] Document each supported public standard-library category separately.
- [x] Cover IO, filesystem, paths, strings and UTF-8, time, regions, Wire, and
      typed errors/results.
- [x] For every public category, document imports, public types, public verbs,
      ownership roles, inputs, outputs, bounds, cleanup, allocation behavior,
      failure values, and target/runtime boundaries.
- [x] Use runnable examples from `examples/` or add small focused examples
      where an existing example is not sufficient.
- [x] Keep transport, operating-system, device, and application-specific
      behavior outside the generic standard-library descriptions.

### Gate 40.7: Compiler and project workflow

- [x] Document `Actus.toml`, `Actus.lock`, source roots, package settings,
      runtime profiles, target selection, and build profiles.
- [x] Document check, strict check, test, format, build, run, object, and
      executable workflows.
- [x] Explain hosted, embedded, and freestanding boundaries in plain language.
- [x] Document native object generation, linking, linker configuration, and
      target-specific requirements.
- [x] Document diagnostics and a step-by-step error investigation order.
- [x] Document compiler boundaries without inventing workarounds or unsupported
      syntax.

### Gate 40.8: Human examples and learning paths

- [x] Add a beginner path from an empty project to a checked executable.
- [x] Add an intermediate path covering ownership, modules, standard-library
      calls, and typed failures.
- [x] Add an advanced path covering generic types, packs, serialization,
      regions, native output, and target profiles.
- [x] Add complete examples with expected commands and observable output where
      the current repository provides such output.
- [x] Explain examples in prose before and after the code.
- [x] Remove examples that rely on unavailable, future, or undocumented syntax.

### Gate 40.9: Agent reference

- [x] Create a concise reading order for agents before source changes.
- [x] Create a source-change checklist covering repository scope, contracts,
      facades, ownership, layout, tests, and documentation.
- [x] Document how to distinguish implemented behavior, documented boundaries,
      design-only material, and compiler work.
- [x] Document evidence requirements for tests, native output, performance, and
      hardware.
- [x] Document stop conditions for compiler limitations, public API changes,
      serialized layouts, ownership changes, and unexpected worktree changes.
- [x] Link the agent reference to the human handbook without making the human
      handbook read like an agent policy document.

### Gate 40.10: Editorial consistency

- [x] Read every handbook file as a human reader.
- [x] Remove duplicated explanations while preserving technical details.
- [x] Replace internal shorthand with defined terms and plain explanations.
- [x] Ensure every code example uses current Actus syntax and naming.
- [x] Ensure every public API example shows its import and relevant ownership
      roles.
- [x] Check that headings, links, code fences, tables, and lists render
      consistently.
- [x] Check that every handbook file is linked from an index or category page.
- [x] Check that every migrated source-guide section has a destination.

### Gate 40.11: Documentation validation

- [x] Run Markdown link and heading checks used by the repository.
- [x] Run Actus formatting and strict checks on every Actus example.
- [x] Run the relevant example and standard-library tests.
- [x] Compare the completed handbook against the source inventory.
- [x] Review all code blocks for syntax, imports, ownership roles, and current
      APIs.
- [x] Update the handbook index and source-to-destination map.
- [x] Update repository documentation references to point to the handbook.

### Gate 40.12: Guide transition

- [x] Decide the final role of `ACTUS_CODING_AGENT_GUIDE.md` after the handbook
      is complete.
- [x] If it becomes a short entry point, preserve links to all handbook
      categories and the agent reference.
- [x] If it remains a compatibility document, state its relationship to the
      handbook without duplicating the full content.
- [x] Remove stale duplicated sections only after the handbook contains their
      complete destination content.
- [x] Update contribution and repository instructions to use the new handbook.
- [x] Record the completed documentation structure in the phase closeout.

## Required outputs

- `docs/guide/README.md`.
- The category directories and handbook files described by this phase.
- A source-to-destination migration map.
- Human learning paths and complete examples.
- An agent reference with source-change and diagnostic checklists.
- Updated links from repository contribution and coding instructions.
- A phase closeout listing completed gates and remaining documentation work.

## Commit structure

Use focused commits:

1. `docs: add Actus handbook structure`
2. `docs: migrate Actus language fundamentals`
3. `docs: migrate Actus ownership and modules guide`
4. `docs: migrate standard library handbook`
5. `docs: add compiler and workflow handbook`
6. `docs: add human examples and agent reference`
7. `docs: validate and transition Actus guide`

Do not combine unrelated compiler, standard-library implementation, or runtime
changes with this documentation phase.


## Phase 40 closeout

The handbook is available at [`docs/guide/README.md`](../guide/README.md).

The former root guide file has been removed from the working tree after its
complete content was split into the category-specific reference files. Its Git
history remains available through the repository history.

Validation completed:

```text
handbook source-section coverage: passed
handbook local-link audit: passed
handbook code-fence audit: passed
repository Actus examples formatter check: passed
cargo test --test documentation: 4 passed, 0 failed
cargo test --test examples_cli: 12 passed, 0 failed
cargo test --test formatter: 30 passed, 0 failed
git diff --check: passed
```

The handbook is organized for human readers first, with the agent reference
kept as a separate operational category.


## Follow-up: Core language reference audit

The first handbook pass is complete, but the core language pages need a
separate depth pass. This follow-up keeps the work ordered and source-driven.
It expands one language subject at a time and does not regenerate the whole
handbook in one change.

### Gate 40.13: Core-language audit baseline

- [x] Compare the `pack`, `Arena`, `struct`, `Array`, `Buffer`, constants,
      generics, roles, and `perform` pages with the current parser, semantic
      checks, native examples, and focused tests.
- [x] Record the missing dedicated `Arena` reader page and the shallow
      `struct` and `pack` explanations as follow-up work.
- [x] Identify and correct the invalid relative link from the expressions page
      to the operator guide.
- [ ] Recheck every language-page example after the subject pages are updated.

### Gate 40.14: Arena handbook

- [ ] Create a dedicated human-facing `Arena` page under the language section.
- [ ] Explain `Arena[N]`, bounded capacity, placement, returned references,
      provenance, cleanup, and the rule that references cannot outlive the
      arena owner.
- [ ] Cover recursive and cyclic examples using the current `Option[abs T]`
      form and link the relevant ownership pages.
- [ ] Document the supported boundary and distinguish an arena from a general
      heap or garbage collector.
- [ ] Reconcile the page with `tests/arena_option.rs`, `tests/arena_graph.rs`,
      `tests/arenas_cli.rs`, and the canonical arena examples.

### Gate 40.15: Struct handbook depth pass

- [ ] Expand the struct guide with field roles, aggregate construction,
      partial-field moves, cleanup, nested structs, generic structs, and
      arena-backed references.
- [ ] Explain `perform` contracts through a complete role and implementation
      example.
- [ ] Add valid and rejected examples tied to current ownership and semantic
      diagnostics.
- [ ] Reconcile the guide with struct, drop, generic, and native ABI tests.

### Gate 40.16: Pack handbook depth pass

- [ ] Expand the pack guide with backing storage, byte width, offsets,
      endianness, overlap and capacity validation, field access, and indexed
      pack arrays.
- [ ] Explain which pack layouts are compile-time contracts and which values
      remain runtime data.
- [ ] Document ownership roles on pack storage and fields without implying
      pointer casts or unchecked memory access.
- [ ] Reconcile the guide with pack semantic tests, native pack examples, and
      the serialization reference.

### Gate 40.17: Constants, generics, roles, and dispatch pass

- [ ] Expand constants with compile-time evaluation boundaries, visibility,
      typed widths, and rejected runtime uses.
- [ ] Reconcile generic type and const-generic examples with current accepted
      domains and diagnostics.
- [ ] Expand roles and `perform` documentation with static dispatch, explicit
      dynamic boundaries, receiver roles, and ABI limitations.
- [ ] Record unsupported or design-only behavior instead of presenting it as
      available syntax.

### Gate 40.18: Core-language editorial validation

- [ ] Verify all language-page links and code-fence languages.
- [ ] Verify that examples use current Actus syntax and match focused tests.
- [ ] Ensure the language index exposes every new subject page.
- [ ] Record the completed audit and remaining limitations in this roadmap.

## Library handbook reorganization

The standard-library section was subsequently reorganized into
`docs/guide/library/`. Each public library now has its own directory and API
inventory. The former `docs/guide/standard-library/` directory has been
removed after its overview pages, reference material, and links were moved.

Current dedicated library directories include:

- `io/`
- `filesystem/`
- `path/`
- `string/`
- `time/`
- `region/`
- `wire/`
- `errors/`

The source library and tests remain the implementation reference for every
API entry.
