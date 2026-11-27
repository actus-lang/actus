# Cleanup and scope

Owned values are cleaned up deterministically at scope boundaries. Cleanup is
planned in reverse creation order for live owners and is also considered on
return, `break`, `continue`, typed failure, and branch exit.

Every exit of a scope releases exactly the owners that are live there: the end
of the block, a `return`, a `break` or `continue`, and the failure exit of
`?`. The scopes being left are released innermost first, and the owners of
one scope in reverse order of declaration. The analyzer computes the list of
each exit; code generation only runs it, so a value moved on one path is never
released on another and there is no hidden run-time drop flag (ADR-0125).

The owners of a verb's scope are its `erg` bindings and its `erg` and `dat`
parameters. A `dat` parameter that the verb neither moves nor drops is released
when the verb returns, so `verb inspect(dat config: Config) { }` still releases
the buffers and runs the `Drop` performances of `config`. Each branch block of
an `if` is a scope of its own.

Releasing a value runs the `Drop` performance of its own type first, then
releases each field it still owns in reverse declaration order, recursively.
A `Buffer` frees its storage. An enum such as `Option[Buffer]` releases the
payload of its active variant only, and an array releases its elements, last
element first. `drop(x)` runs the same release where it is written.

Assigning to an owner, to a field (`config.name = Buffer[8];`, also through an
`ins` loan) or to an array element (`slots[i] = Buffer[8];`) releases the value
the place held before the new one is stored. The assigned expression is
evaluated first, so it can still read the old value; then the old value is
released; then the new one is stored. A field that was moved out holds nothing:
the assignment releases nothing and makes the field owned again. An element is
always owned, so a value that holds a resource cannot be moved out of an
element (`E1010`); borrow it with `abs` or assign a new value in its place.

A value that no binding receives is released as soon as it is no longer read:
the result of a call that a statement discards (`make();`) at the end of the
statement, and a call that a field or an element is read from
(`pair().a.id`) after that read. A `?` nested in an expression
(`Pair { a: track(id: one), b: load()? }`) releases, when it fails, every value
the statement already evaluated: the compiler binds those values and the `?`
to hidden owners in evaluation order, and the exit of the `?` releases the
ones that are live.

A `Drop` performance receives its value through `ins self` and releases what
it owns in place. It must not build or pass on another owner of its own type:
that owner would be released by the same performance again.

A moved value is removed from the previous owner's cleanup plan. A borrowed
value cannot be cleaned up while its view or loan is live. A failed operation
must leave cleanup in the state described by its ownership contract.

Do not add manual cleanup to compensate for an ownership error. Fix the role,
move, borrow scope, or return contract that produces the invalid state.

Resources that cross a foreign-function boundary need a documented cleanup and
status contract. Keep raw bridge details behind a typed public facade.
