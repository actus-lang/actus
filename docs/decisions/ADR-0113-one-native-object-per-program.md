# ADR-0113: One Native Object per Program

## Status

Proposed. Supersedes levels 2 and 3 of ADR-0112.

## Context

ADR-0112 gave every function local linkage unless something outside its
object needed it. Object emission still wrote one object for the root
program and one for every imported unit (`app.obj`,
`app.module.actus_mod_4_math.obj`), so three groups of functions stayed
global:

- public verbs of imported units, which the root object calls;
- generic instances emitted in one object and used by another;
- perform implementations dispatched from another unit.

A host that links Actus objects, such as a C or Rust firmware image on a
freestanding target, could call any of them by symbol name and bypass the
language boundary. The linker and LLVM also could not drop or inline them
across units. Test plans kept every symbol global, because a test reaches
private declarations of another object.

ADR-0112 planned two follow-ups: hidden visibility for public verbs, and a
partial link (`ld -r`) of the objects followed by localizing hidden
symbols. A partial link depends on an external tool, is not available for
COFF, and would undo work the compiler had just done by splitting the
program into objects. No build reuses a unit object: every build emits all
objects again.

## Decision

### One object

The compiler lowers every unit as before, then links the lowered backend
modules into one backend module and emits one relocatable object for the
program, with both backends. References between units resolve inside that
object. An external declaration of the same symbol in two units must have
one signature; an external that the program itself defines is dropped.

`--emit obj` writes that single object. `--emit exe` links it with the
runtime. The object manifest (`.symbols`) lists one object.

### The host contract

The only global symbol the object defines is the program entry: the
configured `entry_symbol`, or the hosted `main`. Every other function has
local linkage, including public verbs, generic instances, perform
implementations, and the verbs of a test plan. The object's undefined
symbols are what the runtime, a provider, or the host must supply.

Calling an Actus verb from a host other than through the entry needs an
explicit host ABI declaration in the language. That is a separate decision;
until it exists, the entry is the whole host surface.

### Hidden visibility

There is no link between Actus objects any more, so hidden visibility has
nothing to hide. It is deferred until Actus emits shared libraries.

## Consequences

- A host can no longer link against any Actus verb except the entry.
- LLVM may inline across units and remove unused functions of every unit.
- Test plans need no linkage exception: a test reaches private declarations
  inside the one object.
- The multi-object contract of `--emit obj` ends: builds produce no
  `*.module.*` sidecar objects. Tests and tools that read sidecars change
  once.
- The `nm` evidence of an object shows one global definition, the entry.
  A test checks this for hosted and freestanding builds.
- Determinism is unaffected: units, functions and externals keep a fixed
  order.
