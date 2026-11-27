# ADR-0125: Static Cleanup at Every Exit

## Status

Accepted.

## Context

ADR-0027 promises deterministic cleanup: every owned resource is released
exactly once, at a point the source text determines. The implementation did
not keep the promise for aggregates (bugs #114, #115, #116, #122, #124, #125):

- a `dat` parameter that the callee neither moved nor dropped was never
  cleaned up, so its buffers leaked and its `perform Drop` never ran;
- a struct owner had no cleanup of its own: neither a scope end nor an
  explicit `drop` released its fields or ran their drop contracts;
- an enum that holds a resource in its payload, such as `Option[Buffer]`, was
  never cleaned up;
- code generation built one cleanup per verb and matched it to bindings by
  name, so a value moved on one path could be released again on another, and
  the blocks that continue after an `if`, a `loop` or a `for` released nothing.

Other languages solve the last point with drop flags: a hidden run-time bit
per value records whether it is still owned, and the cleanup tests the bit.
Actus does not need one. The ownership rules already reject every program in
which a binding may or may not be owned: the branches of an `if` or a `case`
and the exits of a loop must leave each binding in the same state (`E1061`,
ADR-0016). At every point of an accepted program the set of live owners is
therefore known at compile time, and a hidden flag would only hide what the
source already states.

## Decision

### Every exit releases what is live there

A scope is left through one of its exits: the end of its block, a `return`, a
`break` or a `continue`. Each exit releases exactly the owners that are live at
that exit, in this order:

1. the scopes being left, innermost first;
2. inside one scope, the bindings in reverse order of declaration.

The semantic analyzer computes the list for each exit from the same ownership
state that it checks moves with, after the expressions of the exit statement
have been analyzed: a value that the operand of a `return` moves is not live
at that `return`. Code generation executes these lists and derives nothing
itself; a list names its bindings by identity, never by name. There is no drop
flag and no other hidden run-time ownership state.

An exit inside a branch has its own list. A branch that leaves the verb with
`return` after moving a value does not affect the list of the code that
follows the branch, because that code is reached only by the other branches.

### Owners

The owners of a scope are its `erg` bindings and the `erg` and `dat`
parameters of its verb. An `abs` view and an `ins` loan own nothing and are
never cleaned up. A binding that was moved or dropped is not live. A binding
that was moved in part (ADR-0046) is live for the fields it still owns.

### Cleanup of a value

Cleaning up a value means, in this order:

1. its own drop contract (`perform Drop`), if its type has one;
2. its fields that it still owns, in reverse order of declaration, each
   cleaned up by this same rule;
3. for a `Buffer`, releasing its storage; for an `Arena`, releasing the arena.

An enum is cleaned up by its active variant: the variant tag, which the value
already carries, selects the payload to clean up, and a variant without a
resource does nothing. The tag is part of the value, not a hidden flag.
An array is cleaned up element by element, last element first.

`drop(x)` runs the same cleanup at the point where it is written and ends the
ownership of `x`.

A drop contract receives its value through `ins self` and must not take
ownership of it again: a contract that builds or passes on an owner of the
same type would run itself without end. A contract releases the resources it
is responsible for in place.

### Rejected alternative: drop flags

A run-time flag per value would accept programs that move a value on some
paths only. It was rejected because it adds state that no source line shows,
costs memory and a branch at every exit, and makes the point of release depend
on the path taken instead of on the text. A program that needs different
ownership on different paths writes the difference: it drops the value
explicitly on the path that keeps it.

## Consequences

- A `dat` parameter is released at the end of the callee unless the callee
  moves it on. Library code that relied on the leak is corrected.
- Cleanup has one source of truth, the analyzer's exit lists, for the LLVM and
  the Cranelift back end alike.
- Each rule above has native tests on both back ends that run under a memory
  checker and assert no leak, no invalid free and the order of drop output.
- The formal model of ownership (phase 57) proves the claim this decision
  rests on: in an accepted program the live owners at every exit are static.
