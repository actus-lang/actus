# Sequence window

`WireSequenceWindow` is bounded replay suppression for one application
context. It is not authentication and cannot prove who sent a frame.

```actus
struct WireSequenceWindow {
    initialized: Bool,
    context_id: u32,
    highest: u32,
    received: u64,
}
```

The 64-bit mask retains the newest sequence and the previous 63 positions.
The largest accepted forward jump is 1024. Sequence arithmetic is modular
`u32` arithmetic and supports the tested wrap boundary.

## First frame

The first call to `wire_sequence_accept` establishes `context_id`, stores the
sequence as `highest`, sets bit zero, and returns success. A later frame with a
different context returns `SequenceContextMismatch` without changing state.

## Forward sequence

A forward sequence within the maximum jump advances `highest`. If the jump is
at least 64, older receipt history is discarded and only the new sequence
remains marked. Smaller jumps shift the receipt mask and mark the new highest.
A jump beyond 1024 returns `SequenceJumpTooLarge`.

## Older sequence

An older sequence inside the 64-position mask is accepted once. Its bit is
then marked. A marked bit returns `SequenceDuplicate`; a sequence outside the
mask returns `SequenceStale`.

The half-range rule distinguishes modular forward movement from backward
movement, including the `u32` wrap boundary.

## Context lifecycle

- `wire_sequence_reset` clears initialization, context, highest, and mask.
- `wire_sequence_replace_context` installs a new context and clears history.
- `wire_sequence_highest` observes the newest accepted sequence without
  mutation.

Use an explicit reset or context replacement when a session/stream changes;
do not rely on a new context arriving implicitly.
