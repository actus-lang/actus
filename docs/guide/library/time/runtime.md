# Runtime and target behavior

## Monotonic provider

The hosted runtime supplies a private monotonic clock bridge. The public
`monotonic_nanos` wrapper returns a non-decreasing process-local counter in
nanoseconds for the active provider domain. Its origin and hardware
resolution are unspecified.

The counter is suitable for ordering and same-domain elapsed-time subtraction.
It is not a calendar timestamp and must not be used as a portable persisted
identity.

## Provider capabilities

The selected runtime profile determines whether monotonic time and sleep are
available. A compiler accepting the import does not by itself prove that a
freestanding target has a clock or scheduler provider.

A compatible provider must preserve the public rules: integer nanosecond
representation, monotonic ordering, checked conversion, and explicit failure
for unsupported operations.

## Allocation and ABI

`Instant`, `Duration`, `Deadline`, and `Timer` contain bounded scalar state.
The time library does not allocate for arithmetic, deadlines, or timer polls.
The private sleep and clock bridges receive scalar values and do not expose
raw platform structures to Actus code.

## Blocking and measurement

The API does not promise fixed latency. Clock reads, scheduler sleep, and
provider wake-up behavior depend on the target. Benchmarks must record target,
provider, compiler, duration, and scheduling conditions separately from the
source-level timing contract.

## Out of scope

Calendar time, timezone conversion, formatting, synchronization primitives,
callbacks, timer threads, and OS scheduler policy are outside `std::time`.
They require separate explicit libraries or application contracts.
