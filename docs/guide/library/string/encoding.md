# Encoding and UTF-8 rules

## UTF-8 is validated at the boundary

`utf8_from_buffer` and `utf8_from_string` establish the invariant that the
returned `Utf8Buffer` contains valid UTF-8 bytes. Invalid input returns a
typed error and does not become a partially valid value.

`string_length` and `string_byte_at` also report invalid provider encoding as
`StringError.InvalidUtf8`. The caller can therefore distinguish malformed
text from an empty text value or an invalid index.

## Length and indexes count bytes

The library uses UTF-8 storage but exposes byte-level access. A Unicode scalar
may occupy one to four bytes. Therefore:

- `utf8_length` is storage length in bytes;
- `utf8_byte_at` returns one byte in the encoded sequence;
- `string_byte_at` follows the same byte rule for borrowed `String` values;
- an index inside a multibyte sequence is valid as a byte index.

The library does not promise that each returned byte is a complete character.
Applications that need scalar or grapheme operations must add a separate,
explicit layer rather than guessing from one byte.

## Embedded zero bytes

`Utf8Buffer` is length-delimited by its stored `length`. A zero byte inside
the buffer is therefore data and does not terminate the value. This behavior
is different from a C-style null-terminated string provider, where a zero byte
may terminate the source representation.

## No repair or replacement

The library does not replace invalid sequences, drop bytes, normalize text, or
reinterpret arbitrary binary payloads as UTF-8. If validation fails, the
caller receives `StringError.InvalidUtf8` or `StringError.InvalidStorage`,
depending on which contract was violated.

## String and byte output are separate

`std::io` text verbs operate on `String`; byte verbs operate on `Buffer`.
Keep the distinction at the call site. Converting a buffer-backed value to a
text value requires an explicit validated constructor.
