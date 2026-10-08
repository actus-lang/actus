# String representations

## Borrowed `String`

`String` is the runtime text representation accepted by existing text APIs.
The public string functions receive it as `abs text: String`. This means the
operation may inspect the value for the duration of the call, while ownership
stays with the caller.

The library does not expose the hosted pointer ABI. Provider details remain
behind the facade so callers depend on the checked Actus contract rather than
on a platform pointer or terminator convention.

## Owned `Utf8Buffer`

`Utf8Buffer` is a separate public type:

```actus
struct Utf8Buffer {
    storage: Buffer,
    length: Int,
}
```

It represents caller-supplied storage that has passed UTF-8 validation. The
stored length is explicit and is used for bounds checks. It is not inferred
from a later scan at every read.

## Why the representations are separate

The borrowed runtime value and the owned buffer have different lifetime and
storage rules. Keeping them separate makes the following facts visible:

1. inspecting a runtime string does not transfer ownership;
2. retaining text requires explicit destination storage;
3. capacity failures are reported at construction time;
4. binary buffers are not silently treated as text;
5. embedded zero bytes are representable in the length-delimited value.

## Buffer is not a general-purpose text value

`Buffer` can contain arbitrary bytes. It becomes an `Utf8Buffer` only through
`utf8_from_buffer` or `utf8_from_string`. This boundary prevents invalid or
partially copied data from being presented as validated text.

## No hidden growth

The standard library does not grow `storage` and does not allocate a fallback
buffer when the destination is too small. The caller chooses a capacity and
can retry with another buffer after receiving `CapacityExceeded`.
