# `std::time` semantics and runtime

All public duration calculations use integer units and checked arithmetic.
The module does not promise a particular hardware clock frequency or scheduler
precision. The selected runtime provider defines how `now`, `sleep`, and
timer polling obtain time.

`delay` is a busy-wait operation and consumes CPU while it runs. `sleep` is a
provider operation and may block or suspend according to the target. A
freestanding target must supply the provider contract before hosted waiting
operations can be used.

Timer state is changed through `ins`; `Instant`, `Duration`, `Deadline`, and
timer state queries are inspected through `abs`.
