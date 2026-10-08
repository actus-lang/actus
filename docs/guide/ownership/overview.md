# Ownership overview

Actus ownership answers one question at every call: who may change, inspect,
transfer, or clean up this value now?

A binding's role is part of its type contract. The compiler tracks the role
through declarations, calls, fields, array indexes, branches, loops, returns,
and cleanup.

The four source roles are:

- `erg`: the active owner that may mutate its value;
- `abs`: a read-only view that does not take ownership;
- `dat`: a terminal ownership transfer to the callee;
- `ins`: an exclusive, call-scoped mutable loan.

Use the smallest role that describes the operation. A read-only operation should
not request a mutable owner, and a function that consumes a resource should not
accept an `abs` view.

The compiler rejects use-after-move, mutation through a read-only view, aliasing
exclusive loans, invalid cleanup, and ownership states that cannot join after a
branch.
