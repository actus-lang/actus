# Case and branch ownership

Ownership is checked independently on every `if`, `case`, and loop path. Paths
join only when their resulting ownership states are compatible.

A diverging branch such as `return`, `break`, or `continue` does not contribute
a value to an expression branch join, but its cleanup still runs for values
that remain live on that path.

Enum payload moves are branch-local. An `abs` case subject remains a read-only
owner after the case; a `dat` subject may be consumed by the match. A branch
cannot use a payload after moving it, and another branch cannot rely on a move
that happened only in a different branch.

When a branch error appears, compare each branch's final owner, moved, borrowed,
and dropped states. The fix belongs at the first operation that creates the
incompatible state.
