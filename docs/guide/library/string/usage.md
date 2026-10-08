# Using `std::string`

## 1. Inspect a borrowed string

`string_length` returns the UTF-8 byte length without consuming the source.
`string_byte_at` returns one byte at a checked index.

```actus
import std::string;

open verb first_byte(abs text: String) -> Result[Int, StringError] {
    return string_byte_at(text: abs text, index: 0);
}
```

Both calls preserve `text`. The result must be handled as a `Result`; a
negative index, an index beyond the byte length, a null provider value, or
invalid source encoding is not converted into a sentinel byte.

## 2. Copy into caller-owned storage

The destination is supplied explicitly. Its capacity is part of the caller's
design and must account for the UTF-8 bytes that will be copied.

```actus
import std::string;

open verb copy_text(abs source: String, dat storage: Buffer)
    -> Result[Utf8Buffer, StringError] {
    return utf8_from_string(text: abs source, storage: dat storage);
}
```

The source remains borrowed. The destination buffer is consumed by the call;
on success it is owned by the returned `Utf8Buffer`, and on failure the
library performs deterministic cleanup before returning the error.

## 3. Validate an existing buffer

Use `utf8_from_buffer` when the bytes are already in a `Buffer` and should be
treated as UTF-8 data.

```actus
import std::string;

open verb validate_text(dat storage: Buffer)
    -> Result[Utf8Buffer, StringError] {
    return utf8_from_buffer(storage: dat storage);
}
```

The buffer must contain valid UTF-8 according to the provider. Length is
stored explicitly, so an embedded zero byte is allowed in this representation.

## 4. Read an owned value

`utf8_length` returns the stored byte length. `utf8_byte_at` reads a byte from
the owned value through an `abs` view:

```actus
open verb byte_at(abs value: Utf8Buffer, erg index: Int)
    -> Result[Int, StringError] {
    return utf8_byte_at(text: abs value, index: erg index);
}
```

These operations do not transfer or mutate the backing storage. They still
return `InvalidStorage` if an invalid aggregate reaches the public boundary.

## 5. Handle the result explicitly

The exact `case` shape depends on the surrounding operation, but callers must
preserve both branches:

```actus
erg length_result = string_length(text: abs text);
return case length_result {
    Ok(length) => Ok(length),
    Err(error) => Err(error),
};
```

Do not treat `0` as an error and do not interpret an error enum as a byte
length. An empty valid string has length zero.

## Byte indexing is not character indexing

For the UTF-8 text `é`, the byte length is two. Index `0` and index `1` return
the two encoded bytes. The API does not provide Unicode scalar decoding,
normalization, case conversion, grapheme segmentation, or locale behavior.

## Choosing output APIs

Pass `String` to text-oriented `std::io` operations. Pass a `Buffer` to byte
output operations such as `printb` or `printlnb`. A buffer that happens to
contain printable bytes is still not automatically a `String`.
