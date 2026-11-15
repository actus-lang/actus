# ADR-0112: Symbol Linkage Follows Declaration Reach

## Status

Proposed. Level 1 is implemented; ADR-0113 supersedes levels 2 and 3
and the test-plan exception.

## Context

ADR-0018, ADR-0110 and ADR-0111 decide how far an Actus declaration
reaches: its directory module, its unit (`open unit x;`), or every unit
(`open x;` on the whole facade path). Object emission ignored that reach.
Both backends gave every function a global symbol, so a private verb was as
visible to the linker as a public one:

- a C or Rust host linked with an Actus object could call a private verb by
  its symbol name and bypass the language boundary;
- neither the linker nor LLVM could drop or freely inline a function that
  another object might still call, which costs flash on microcontrollers;
- the symbol table of an object listed every internal function, so its real
  surface was everything it defined.

## Decision

Linkage is derived from reach. The decision is taken once per program,
after shared lowering, from the lowered modules of every object; both
backends only apply it.

### Level 1: global only when something outside the object needs it

A function keeps a global symbol when it is:

1. the program entry;
2. a public verb of an imported unit (every facade on its path opens it
   with `open x;`); or
3. named by another object of the program: a call, a cleanup performance
   bridge, or an external declaration. This covers perform implementations
   dispatched from another unit and generic instances emitted in one object
   and used by another.

Every other function, including private verbs, `open unit` verbs and
specializations used only inside their object, gets local linkage:
`Linkage::Local` in Cranelift and `internal` in LLVM.

### Test plans

A test plan reaches private declarations of its module across objects
(friend scope), so every function of a test build keeps a global symbol.
Test binaries are not shipped. When grouped test binaries (roadmap T) build
a test with its unit in one object, this exception can be removed.

### Levels 2 and 3 (superseded by ADR-0113)

ADR-0113 emits one object per program instead, so no Actus symbol links
between objects and the entry is the only global definition. The plan below
is kept for the record.

- Level 2: public verbs get hidden visibility, so they link between Actus
  objects without entering a shared library's export table.
- Level 3: the native module handed to a host is one relocatable object
  (partial link, then hidden symbols localized), whose only global symbols
  are the entry and explicit host ABI declarations. Its symbol table becomes
  a contract checked by a gate.

## Consequences

- A host can no longer link against a private or unit-scoped verb.
- LLVM may remove unused internal functions and inline local ones.
- The `nm` evidence baseline changes once: internal functions appear as
  local (`t`) instead of global (`T`). Determinism is unaffected.
- The linkage decision is exact: a symbol is global only because of a
  stated reason, and the reasons are computed from the lowered program,
  not from naming conventions.
