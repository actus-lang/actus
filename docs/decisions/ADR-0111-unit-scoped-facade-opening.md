# ADR-0111: Unit-Scoped Facade Opening

## Status

Proposed

## Context

ADR-0018 makes a facade the contract of a directory module: `open x;` in a
facade exports the `open` declarations of sibling file or child directory
`x`. ADR-0110 makes every directory of a unit a module with a hierarchical
scope, so a declaration shared between two branches of a unit (for example
a binary codec used by both `pkg::store::shard` and `pkg::store::log`) must
be reachable from the other branch through an import.

Today the only way to reach it is to export it. Every facade on the path
opens its child, so an exported declaration also becomes part of the unit's
public API and reachable from every other unit. Projects then either place
internal helpers in a common ancestor or publish them. Actus has no level
between "this directory module" and "everyone".

## Decision

### Syntax

A facade may open a sibling file or child directory for its unit only:

```act
// src/pkg/store/shard/shard.act
open fixed;          // public: reachable from other units
open unit codec;     // unit-scoped: reachable only inside unit `pkg`
```

`unit` is a contextual word. It is a modifier only between `open` and a
name: `open unit;` still opens a file or directory named `unit`, and
`open unit unit;` opens it for the unit. `unit` is not reserved anywhere
else.

The visibility boundary stays in the facade. A declaration keeps the single
marker `open`, which means "leaves its directory module"; how far it
reaches is decided only by facades. There is no unit-scoped marker on a
declaration.

### Reach of a name

A name is exported by a directory module at one of two levels:

- **public** when the declaration is `open` and every facade from its file
  up to the unit root opens the next step with `open x;`;
- **unit** when the declaration is `open` and at least one facade on that
  path uses `open unit x;`.

A parent that opens a child with `open x;` keeps each of the child's
unit-scoped names unit-scoped; a parent that opens a child with
`open unit x;` makes all of the child's names unit-scoped. A name never
becomes public above a unit-scoped step.

One facade may not open the same name both ways (`E1137`).

### Use

- Inside the unit, unit-scoped names follow ADR-0110 exactly as public
  names do: they are reachable through an import of the module that
  exports them, as exports of an opened child, and through qualification.
- Outside the unit, unit-scoped names are not exports. A reference from
  another unit is a private declaration access (`E1109`), exactly as for a
  declaration that no facade opens. An import of the unit's child module
  (ADR-0110) reaches only public names: a unit-scoped name through it is
  `E1119`, and a child directory opened with `open unit` cannot be imported
  from another unit at all (`E1108`).
- A test plan's friend modules see unit-scoped names of their unit, as they
  see its private declarations today.

### Code generation and tooling

Unit scope is a frontend rule: object planning, symbols and linkage are
unchanged, as they already are for declarations that no facade opens.
Narrowing the linkage of every non-public verb is a separate decision.

The formatter prints `open unit x;` among the facade's openings with the
same spacing as `open x;`, and the language-server formatting edit keeps
the modifier.

## Consequences

- Internal helpers shared across branches of a unit stay out of its public
  API without moving them to a common ancestor.
- The public surface of a unit can be read from its facades alone.
- `E1137` is a new diagnostic for a facade that opens one name both ways.
- Existing sources are unchanged: `open x;` keeps its meaning, and no word
  becomes reserved.
