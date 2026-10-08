# Constants and compile-time data

A named constant gives a compile-time value a stable name:

```actus
const MAX_PAYLOAD_BYTES: u16 = 1024u16;
```

Constants are immutable values. They do not own resources, allocate memory,
read runtime state, call verbs, or cross a foreign-function boundary.

A constant expression may use supported typed primitive operations and other
constants when evaluation is deterministic. The compiler rejects cycles,
unknown names, incompatible types, and values outside the declared range.

Constants are private by default. An exported constant follows the same
canonical facade and public-documentation rules as any other public
 declaration.

Use constants for protocol limits, masks, offsets, and other values whose name
carries meaning. Keep the declared width aligned with the file, wire, register,
or ABI contract that uses it.
