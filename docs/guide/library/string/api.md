# `std::string` public API

This page describes the public facade contract. The raw provider declarations
in the implementation are private and are intentionally omitted from the
public API.

## Types

### `StringError`

```actus
enum StringError {
    Null,
    InvalidUtf8,
    OutOfBounds,
    InvalidStorage,
    CapacityExceeded,
    ProviderUnavailable,
}
```

`Null` identifies null provider input. `InvalidUtf8` identifies malformed
encoding. `OutOfBounds` identifies an invalid byte index or unrepresentable
length. `InvalidStorage` identifies inconsistent buffer metadata or rejected
UTF-8 storage. `CapacityExceeded` means the caller's destination is too
small. `ProviderUnavailable` identifies a runtime without the required
string provider.

### `Utf8Buffer`

```actus
struct Utf8Buffer {
    storage: Buffer,
    length: Int,
}
```

The aggregate owns validated caller-provided storage. `length` is the logical
UTF-8 byte length and is checked before the value is accepted or read.

## Borrowed access

```actus
verb string_length(abs text: String) -> Result[Int, StringError];
verb string_byte_at(
    abs text: String,
    erg index: Int,
) -> Result[Int, StringError];
```

Both operations preserve the source. `string_length` returns a byte count;
`string_byte_at` returns one encoded byte. Negative and out-of-range indexes
return `OutOfBounds`.

## Construction

```actus
verb utf8_from_buffer(
    dat storage: Buffer,
) -> Result[Utf8Buffer, StringError];

verb utf8_from_string(
    abs text: String,
    dat storage: Buffer,
) -> Result[Utf8Buffer, StringError];
```

Both constructors use caller-provided storage. The buffer is consumed by the
call. `utf8_from_buffer` validates the existing bytes. `utf8_from_string`
copies the borrowed source, checks provider status and capacity, then applies
the same UTF-8 validation.

## Owned access

```actus
verb utf8_length(
    abs text: Utf8Buffer,
) -> Result[Int, StringError];

verb utf8_byte_at(
    abs text: Utf8Buffer,
    erg index: Int,
) -> Result[Int, StringError];
```

Owned access borrows the aggregate and does not transfer its storage. It
returns `InvalidStorage` for an invalid aggregate and `OutOfBounds` for an
invalid index.

## Result mapping

| Operation | Success | Typed failures |
| --- | --- | --- |
| `string_length` | UTF-8 byte length | `Null`, `InvalidUtf8`, `OutOfBounds` |
| `string_byte_at` | one encoded byte | `Null`, `InvalidUtf8`, `OutOfBounds` |
| `utf8_from_buffer` | owned validated value | `InvalidStorage` |
| `utf8_from_string` | owned copied value | `InvalidStorage`, `CapacityExceeded`, `InvalidUtf8` |
| `utf8_length` | stored byte length | `InvalidStorage` |
| `utf8_byte_at` | one encoded byte | `InvalidStorage`, `OutOfBounds` |

The implementation may receive private integer status codes from a provider,
but those codes do not cross the public facade.
