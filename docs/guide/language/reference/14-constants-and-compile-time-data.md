# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 14. Constants and compile-time data

Named constants are compile-time values:

```act
const MAX_FRAME: u16 = 1024u16;
const HEADER_SIZE: u8 = 8u8;
const DATA_MASK: u32 = 0x00ff_ffffu32;
```

Constants are intended for protocol masks, limits, widths, layout values, and
other data that should not become mutable runtime storage. The compiler
validates constant types, ranges, dependencies, cycles, and compile-time
availability. Do not use a runtime variable when a protocol invariant is
truly fixed.

Compile-time constants do not make arbitrary runtime computation compile-time.
Do not rely on a constant evaluator to read files, call foreign functions,
inspect hardware, or access mutable state.

Pack field offsets may name a checked integer constant:

```act
const PAYLOAD_OFFSET: u16 = 8u16;

pack Frame {
    erg storage: Array[u8, 2];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg payload: u8 at PAYLOAD_OFFSET;
    }
}
```

Named offsets may chain through integer constants and checked integer
arithmetic. They must resolve before native lowering and remain within the
`u16` layout-offset domain. Calls, runtime reads, allocation, mutation,
buffers, strings, booleans, floating point, and dynamic lengths are invalid in
layout constants. The numeric `at 0` form remains valid for literal offsets.
