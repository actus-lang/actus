# `meta bounded(...)`

`meta bounded` declares statically verifiable resource limits for a verb. Every
value is an explicit, non-negative integer literal with a width matching the
field. Zero is meaningful when a resource must not be consumed.

```actus
meta bounded(max_iterations: 1024u32)
verb process(erg state: State) -> Int {
    for erg index: u32 in 0u32..8u32 {
        step(state: ins state, index: erg index);
    }
    return 0;
}
```

The complete vocabulary is `max_iterations`, `max_call_depth`, `max_calls`,
`max_allocations`, `max_allocation_bytes`, `max_stack_bytes`, and
`max_io_bytes`. The parser rejects unknown, duplicate, unsuffixed, or
out-of-width fields.

## Accepted budget model

The compiler calculates a conservative worst-case budget. Sequential work is
added, branches use the largest reachable branch, and nested loops multiply
their proven iteration counts by the largest body budget. Fixed-array loops use
the array's static capacity. Calls are counted conservatively at the call site
and repeated for every proven loop iteration.

All budget arithmetic is checked. Overflow, an unknown bound, or a computed
budget greater than `max_iterations` is a compile-time error.

## What is not accepted

- an ordinary unbounded `loop`;
- a range loop with a runtime-dependent endpoint whose maximum is not proven;
- `meta limitless` as an exemption.

The compiler rejects these cases rather than inserting a runtime counter or
guessing a safe value.

## Relationship to realtime

The semantic pass checks all declared dimensions for the source-level facts it
can prove: fixed buffer allocations, statically known local scalar sizes, and
declared filesystem/network calls. Unknown allocation, layout, or I/O size is
rejected rather than guessed. Native and imported boundaries must expose enough
metadata for the selected budget to remain provable.

`bounded` and `realtime` are independent. `bounded` concerns explicit resource
budgets;
it does not remove `blocking`, filesystem, network, allocation, clock, entropy,
or other effects. A realtime verb may require both attributes.

The contract also does not prove machine execution time, instruction count,
cache behavior, interrupt latency, scheduler behavior, or native code cost.

`bounded` changes no ownership, cleanup, ABI, serialization, or runtime
semantics. It is a compile-time proof contract only.
