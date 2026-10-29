# ADR-0095: Proof-carrying bounded execution contracts

- Status: Accepted
- Date: 2026-10-29
- Scope: `meta bounded(...)`, static resource budgets, native boundaries, and
  diagnostics

## Decision

Actus will support `meta bounded(...)` as a compile-time resource contract.
Fields are explicit, width-checked, non-negative `u32` or `u64` literals. The
attribute is a proof obligation, not a runtime counter, optimization hint, or
approximate performance estimate.

```actus
meta bounded(max_iterations: 1024u32)
verb process(erg state: State) -> Int {
    for erg index: u32 in 0u32..8u32 {
        step(state: ins state, index: erg index);
    }
    return 0;
}
```

The stable vocabulary is `max_iterations`, `max_call_depth`, `max_calls`,
`max_allocations`, `max_allocation_bytes`, `max_stack_bytes`, and
`max_io_bytes`. A field is never silently ignored: it may be accepted only
when its proof boundary is implemented, otherwise compilation must explain the
missing proof.

## Budget algebra

The semantic checker computes a conservative worst-case budget:

- sequential statements sum their proven budgets;
- branches use the maximum budget of any reachable branch;
- a statically bounded range loop contributes its proven maximum iteration
  count multiplied by the worst-case body budget;
- a fixed-array loop contributes its fixed capacity multiplied by the worst-
  case body budget;
- nested loops compose through the same multiplication rule;
- a call contributes one call and one call-depth level;
- calls repeated by a proven loop multiply those costs;
- all arithmetic is checked for overflow and rejects on overflow;
- an empty or non-executing path contributes zero.

The semantic pass enforces every declared dimension for the source-level facts
it can prove: fixed buffer allocations, statically known local scalar sizes,
and declared filesystem/network calls. Unknown allocation, layout, or I/O size
is rejected rather than guessed. A runtime-dependent range endpoint is not
treated as a proof of a finite bound merely because it is an integer.

## Recursion and call graph

Direct and mutual recursion require an explicit finite recursion contract.
Until the recursion-bound form is implemented and checked, any recursive path
reachable from a bounded verb is rejected. Imported, generic, dynamic, and
external calls cannot hide an unbounded path.

An external or native declaration must explicitly provide a compatible bound
contract before it can be called from a bounded verb. If the native boundary
cannot provide a proof, the call is rejected; the compiler never guesses a
bound and never inserts an implicit runtime limiter.

## Relationship to other contracts

`bounded` does not imply `realtime`, determinism, absence of allocation, or
absence of blocking. Those properties remain separate effect and context
contracts. A verb may therefore be bounded but blocking, or realtime and
bounded only when both declarations are present and both checks pass.

`meta limitless` cannot disable bounded checking. Ownership, cleanup, ABI,
serialization, and runtime semantics remain unchanged.

## Proof boundary

This contract proves source-level control-flow iteration counts only. It does
not claim a machine-level WCET, instruction count, cache latency, scheduler
latency, interrupt latency, or native implementation cost. Those require a
target-specific measurement contract and are outside this metadata's meaning.

## Required implementation evidence

Implementation must include parser and duplicate checks; constant and overflow
validation; sequential, branch, range, array, and nested-loop fixtures;
unknown-bound and over-budget negatives; direct and mutual recursion tests;
imported, generic, dynamic, and native boundary tests; formatter and LSP
coverage; stable diagnostics; and proof that no runtime counter or ownership
behavior is introduced.
