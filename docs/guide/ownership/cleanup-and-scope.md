# Cleanup and scope

Owned values are cleaned up deterministically at scope boundaries. Cleanup is
planned in reverse creation order for live owners and is also considered on
return, `break`, `continue`, typed failure, and branch exit.

A moved value is removed from the previous owner's cleanup plan. A borrowed
value cannot be cleaned up while its view or loan is live. A failed operation
must leave cleanup in the state described by its ownership contract.

Do not add manual cleanup to compensate for an ownership error. Fix the role,
move, borrow scope, or return contract that produces the invalid state.

Resources that cross a foreign-function boundary need a documented cleanup and
status contract. Keep raw bridge details behind a typed public facade.
