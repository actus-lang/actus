# ADR-0110: Hierarchical Module Scope

## Status

Proposed

## Context

ADR-0018 makes a directory one module whose direct `.act` files share one
scope, and makes a nested directory a separate module. The compiler did not
enforce the second half: a unit merged its facade, its siblings and every
child directory opened by a facade into one flat program, so any file of the
unit could name any declaration of any directory of the unit, private
declarations of unrelated directories included.

The flat program also had no notion of which unit owns a directory. An
import written inside a unit that named one of the unit's own directories,
such as `import { pkg::runtime }` in `pkg/runtime/training/helper.act`,
loaded `pkg::runtime` a second time as an independent unit. That unit held
only the `pkg::runtime` subtree, so a declaration it used from a sibling
directory (`pkg::layout`) was missing and resolution failed with an
unrelated diagnostic (`E1052`, the R2 reproduction of Phase 53).

## Decision

### Scope of a directory module

Every source file belongs to exactly one directory module: the unit's own
facade and siblings belong to the unit's module path, and the files of an
opened child directory belong to that child's module path. A directory is
part of its unit when it is the unit root or when its parent is part of the
unit and the parent's facade opens it. Directories that no facade opens are
not compiled into the unit.

An unqualified spelling in module `M` may name a declaration `D` of the same
unit only when one of these holds:

1. `D` is declared by `M` (any file of the directory, private included);
2. `D` is declared by an ancestor of `M` (private included), as Rust lets a
   child module use the private items of its ancestors;
3. `D` is exported by a child directory that `M` opens, where a directory
   exports the `open` declarations of the sibling files its facade opens and,
   recursively, the exports of the child directories its facade opens; or
4. an `open` or selecting import written in a file of `M` names a module `Q`
   of the unit and `D` is exported by `Q`, or `Q` is `M` or an ancestor of
   `M` and declares `D`.

Sibling and cousin directories therefore require an explicit import, and a
child directory's private declarations are not visible to its parent.

A qualified spelling `binding::name` in `M` uses a binding introduced by an
import written in `M`. When that import names a module of the same unit, the
same rule 4 decides whether `name` is reachable through it.

Imports are not inherited: an import written in a file of module `P` does not
bring names into a child of `P`. A name supplied only by an import of another
module of the unit is rejected.

Directory files share one scope as ADR-0018 already requires; this decision
does not add imports between files of one directory.

### Imports of the unit's own modules

An import that names the unit's own module path or one of its member
directories binds to the unit that already compiles that module. It never
loads a second unit. An `open` or selecting import contributes no
declarations, because the unit already holds them, and only takes part in the
scope rule above. A namespace import introduces `binding::verb` aliases to
the unit's own verbs that the imported module declares or exports, so a
qualified call compiles to the unit's own definition. Two imports of one unit
that bind one name to different modules are an `E1115` import binding
collision.

### Imports of another unit's child module

An import written outside a unit that names one of the unit's child
directories, such as `import pkg::store::cache`, binds to the owning
unit, which is the shortest ancestor of the path that resolves as a module
with a facade. Every facade from the owning unit down to the child must open
the next directory; otherwise the import bypasses a facade (`E1108`). The
import becomes a binding of the owning unit under the same name, and only the
child's exports are reachable through it: a namespace import accepts
`cache::name` only for an export of the child, an `open` import selects
exactly the child's exports, and a selecting import may select only those.
Anything else is `E1119`. The child is never loaded as a unit of its own, so
its declarations keep one canonical identity (ADR-0108) and one copy of their
code. In a test plan, friendship granted to a child module extends to its
owning unit.

### Declaration identity

Canonical declaration identity (ADR-0108) keeps the owning unit's module
path. Within one program every declaration name is unique (`E1106`, `E1110`),
so the unit path already identifies a declaration without ambiguity; the
directory module is a scope attribute, not a part of the identity. Native
symbols, object ownership and the memoized frontend queries of ADR-0109 are
unchanged.

### Diagnostic

`E1119` reports a reference outside the hierarchical scope of its module. The
message names the referencing module, the spelling, and the cause:

- declared by a module that is not in scope, with the import that brings it
  into scope (`import { open <module> }`);
- private to a child module, with the facade change that exports it; or
- imported by another module of the unit, because imports are not inherited.

The check runs once per compilation plan after every unit is loaded, before
declaration annotation, so `actus check`, `actus build`, `actus test` and the
language server report the same diagnostic.

## Consequences

- Module boundaries inside a unit are enforced instead of documented only.
- Sources that relied on the flat unit scope need imports between sibling
  directories, and a parent that used a private declaration of a child must
  export it. Ancestor declarations remain usable without imports.
- An import of a module of the same unit no longer creates a duplicate unit,
  so declarations of sibling directories are always resolved in the complete
  unit (fixes Phase 53 R2).
- An import of another unit's opened child no longer loads a second unit
  that duplicates the child's declarations and code.
- A unit still rejects one declaration name in two of its directories; making
  names directory-scoped would change declaration identity and native symbols
  and needs a separate decision.
