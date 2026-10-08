# `std::string` examples

## Validate a buffer

```actus
import std::string;

open verb validate(dat storage: Buffer)
    -> Result[Utf8Buffer, StringError] {
    return utf8_from_buffer(storage: dat storage);
}
```

The returned value owns the validated buffer. The caller must handle the
`Err` branch; invalid storage is not returned as an apparently usable value.

## Copy a borrowed string

```actus
import std::string;

open verb retain(abs source: String, dat destination: Buffer)
    -> Result[Utf8Buffer, StringError] {
    return utf8_from_string(text: abs source, storage: dat destination);
}
```

The source is read through `abs`. The destination is consumed and becomes the
storage of the result only after the provider reports a successful copy and
the resulting bytes pass UTF-8 validation.

## Inspect a byte safely

```actus
import std::string;

open verb inspect(abs value: Utf8Buffer, erg index: Int)
    -> Result[Int, StringError] {
    return utf8_byte_at(text: abs value, index: erg index);
}
```

This returns an encoded byte, not a decoded character. A caller that needs to
interpret a sequence must first define a separate decoding contract.

## Preserve an empty value

An empty valid string has length `0`. It is not an error and must not be
confused with `Null`, `InvalidUtf8`, or `InvalidStorage`.

## Embedded zero byte

The buffer-backed representation can contain bytes equivalent to `0u8` in
the middle of the value. Its explicit length keeps the remaining bytes
visible to `utf8_length` and `utf8_byte_at`.

## Forward a result

```actus
open verb length_of(abs source: String) -> Result[Int, StringError] {
    erg result = string_length(text: abs source);
    return case result {
        Ok(length) => Ok(length),
        Err(error) => Err(error),
    };
}
```

Forwarding the typed result preserves the distinction between valid zero,
invalid encoding, provider failure, and bounds failure.
