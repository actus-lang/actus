# `meta realtime`

`meta realtime` declares that a verb must remain suitable for a bounded,
latency-sensitive execution path. It is checked by the compiler; it is not a
comment, optimization hint, or permission grant.

```actus
meta realtime
verb advance(erg state: State, abs input: Input) -> Int {
    for erg index: u32 in 0u32..8u32 {
        update(state: ins state, input: abs input, index: erg index);
    }
    return 0;
}
```

## Rejected operations

A realtime verb cannot call a verb declared with any of these effects:

- `blocking`
- `clock`
- `filesystem`
- `network`
- `allocation`
- `entropy`

The restriction is transitive through imported signatures, facades, generic
calls, external declarations, and role dispatch. The caller must therefore
remain valid even when the implementation is reached indirectly.

The compiler also rejects:

- an unbounded `loop`;
- floating-point literals or operations;
- floating-point parameter, return, local, or call-result types;
- direct self-recursion without a statically declared depth bound;
- a role implementation that omits `meta realtime` when the role method has
  declared it.

Range loops and loops over fixed arrays are accepted because their iteration
source is bounded by the language's static model. A runtime-dependent loop
must be expressed with a statically bounded form or remain outside a realtime
verb.

## What the attribute does not prove

The attribute does not calculate worst-case machine instruction time. It does
not prove cache behavior, interrupt latency, scheduler behavior, or the
internal behavior of native code beyond its declared effects. Native
boundaries must provide accurate effect metadata.

`meta realtime` does not change ownership, cleanup, ABI, serialization, or
runtime semantics. It adds compile-time rejection only. `meta limitless` does
not exempt a realtime verb from these checks.

Diagnostics:

- E1125 — forbidden effect in realtime code;
- E1126 — unbounded `loop`;
- E1127 — floating-point use;
- E1128 — missing realtime role implementation contract.
- E1129 — unbounded direct recursion.
