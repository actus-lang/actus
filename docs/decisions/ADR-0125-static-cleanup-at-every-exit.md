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

An arena is released in one bulk step (ADR-0030): it is emptied, so every
placement in it ends at once. Its storage is a fixed slot of the verb on every
back end, emptied when the arena is built and when it is released, so an arena
declared in a loop reuses the same storage on each iteration. A value placed in
an arena is an owner of its own and is released by its binding like any other
owner. A large aggregate (64 KiB or more) is held in a heap block of its own;
when its value moves by being copied into the storage of another owner (a
struct literal field, a field or an element it is stored into), its block is
freed after the copy, and the destination releases the parts.

An enum is cleaned up by its active variant: the variant tag, which the value
already carries, selects the payload to clean up, and a variant without a
resource does nothing. The tag is part of the value, not a hidden flag.
An array is cleaned up element by element, last element first.

`drop(x)` runs the same cleanup at the point where it is written and ends the
ownership of `x`.

### Assignment releases the value it replaces

An assignment to an owner, to a field of an owner or of an `ins` loan
(`c.b = ...`, `c.inner.b = ...`) or to an array element (`xs[i] = ...`)
releases the value the place held, by the rule above, before it stores the new
one. The order is fixed: the assigned expression is evaluated first, then the
old value is released, then the new value is stored. An expression that reads
the old value, such as `p.a = Tracker { id: next(t: abs p.a) }` or a struct
literal assigned to the whole owner, therefore reads it before it is released.
The analyzer records the release of each such assignment from the ownership
state after the assigned expression, like the other lists:

- a field that was moved out before (`sink(b: dat c.b); c.b = Buffer[4];`, or
  `c.b = grow(b: dat c.b)`) holds nothing, so nothing is released, and the
  assignment makes the field owned again; the owner is whole again once no
  moved path is left;
- a field of which a part was moved out releases the rest without running its
  own drop contract, as a partially moved owner does, and is whole again;
- an assignment into a value that was moved out (`c.inner.b = ...` after
  `c.inner` moved) is a use after move;
- an array element is always owned, at a constant and at a run-time index
  alike: the language does not track moves of single elements, so moving a
  value that holds a resource out of an element (`erg b = xs[i]`, a `dat` or
  `erg` argument, a field initializer, a payload, a return, a branch value)
  is rejected with `E1010`. The element can be borrowed with `abs` or replaced
  in place by an assignment.

### Temporaries

A value that an expression produces and no binding receives is a temporary.
It is released, by the rule above, as soon as the expression that last reads
it has been evaluated:

- the result of a call that a statement discards (`make();`, also in a branch)
  is released at the end of that statement;
- a call that a field or an element is read from (`pair().a.id`,
  `filled()[0u32]`) is released after the outermost read of the path.

The analyzer records each temporary with the expression after which it is
released; code generation keeps the temporary's value until then and runs the
release there. A `case` on a call (`case dat make()`) and a view of a call
(`f(x: abs make())`) are rejected (`E1050`, `E1016`), so they produce no
temporary.

### A `?` nested in an expression

A `?` inside an expression (`Pair { a: track(id: one), b: load()? }`,
`count()? + count()?`) leaves the verb in the middle of a statement. Before
analysis the statement is written as the same steps one per statement: each
operand that is evaluated before the `?` and produces a value is bound, in
evaluation order, to a generated `erg` local, then the `?` itself is bound to
one, and the statement reads those locals. The order of evaluation does not
change. The generated locals are ordinary owners, so the exit list of the `?`
releases exactly the values already evaluated, and a generated local the
statement moves is not released again. This applies to the operands that are
always evaluated: the payload of an enum constructor, the fields of a struct
literal, the operands of an arithmetic or comparison operator, the left
operand of `&&` and `||`, the condition of an `if` (the first one of an `else
if` chain), the subject of a `case`, the bounds of a `for`, the value of a
compound assignment and the operand of a field read, an index, a cast, a
borrow or a unary operator. A value that a nested `?` yields lives in its
generated local until the end of the scope, like any owner.

A `?` that runs on one path only is not moved off its path (bug #130). `l &&
r` with a `?` in `r` is written `if l { r } else { false }`, and `l || r` is
written `if l { true } else { r }`, so `r` is still evaluated only when `l`
does not decide the result. A conditional with a `?` in an arm is bound as a
whole to a generated local, after its condition, unless it is already the
value of the statement. Each arm is then a block of its own: an arm written as
an expression becomes a block, an arm whose value is a `?` binds it to a
generated local (`{ erg v = load()?; v }`), and the block is rewritten like
any other, so every `?` is a statement of the arm. An `else if` whose
condition or arms hold a `?` becomes an `else` block holding the inner `if`,
so its condition is evaluated, and its `?` returns, only when the earlier
conditions failed. The lowering splits an arm at each `?` statement: the
`Err` path returns with the exit list of that `?`, which releases the owners
of the arm and of every enclosing scope, and the `Ok` path continues the arm,
whose value reaches the merge from its last block. The arm that is not taken
runs nothing, on both back ends.

What cannot return early from where it stands is rejected with `E1016`
instead of being lowered: a `?` in an argument of a verb, a method or an
intrinsic (an argument must be a place its role binds, with or without a `?`),
in a `case` guard, in the index of an assigned element, and in a conditional,
a loop or an `if` statement nested in the arm of a conditional whose value an
owner receives (those arms are lowered statement by statement). The fix is to
bind the value first, `erg value = load()?;`. The expression lowering no
longer accepts a `?`: one that reached it would use the whole `Result` as its
value and never return, so it is an internal error.

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
