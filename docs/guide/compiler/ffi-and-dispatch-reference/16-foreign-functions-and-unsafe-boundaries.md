# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 16. Foreign functions and unsafe boundaries

Foreign declarations require an explicit boundary:

```act
unsafe extern "C" verb device_read(ins buffer: Buffer) -> Int;
```

`unsafe` tells the compiler and reviewer that safety depends on an external
contract. `extern "C"` selects the C ABI. The initial supported C boundary is
restricted and layout-checked; current documented support includes `Int` and
`Buffer` pointer-like contracts, with role-specific restrictions for `abs`,
`dat`, and `ins`.

Rules for an external bridge:

1. Document the exact ABI, status values, ownership role, and side effects.
2. Keep the raw bridge private behind a typed Actus facade whenever possible.
3. Translate raw statuses into `Result[T, E]` before exposing the API.
4. Never encode errors through undocumented arithmetic such as `status - 1`.
5. Validate pointer/resource handles and layout assumptions at the boundary.
6. Add accepted and rejected native execution tests.

The standard library uses this pattern: private `unsafe extern "C"` bridges,
public typed wrappers, and facade-controlled exports.
