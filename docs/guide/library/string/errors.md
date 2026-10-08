# String errors

`std::string` converts provider and storage failures into `StringError` so a
caller can handle them without depending on raw ABI status integers.

## `Null`

The hosted provider received null string state. This is different from an
empty valid string, whose length is zero.

## `InvalidUtf8`

The source bytes are not valid UTF-8. The library does not replace invalid
sequences or return a truncated value.

## `OutOfBounds`

The requested byte index is negative or beyond the value, or the provider
cannot represent the requested length. It is also the correct result for an
index equal to the byte length.

## `InvalidStorage`

The buffer metadata, stored length, or provider storage contract is invalid.
Construction drops rejected storage; owned reads report the error without
mutating the value.

## `CapacityExceeded`

`utf8_from_string` could not fit the source bytes in the caller's destination
buffer. The API does not allocate or silently truncate. Supply a larger
caller-owned buffer and retry if the application permits retrying.

## `ProviderUnavailable`

The selected runtime does not provide the required string inspection service.
This variant documents the freestanding boundary; a target integration must
provide the service before claiming support for the facade.

## Handling rule

Always branch on `Result` and preserve the error. Do not map every failure to
an empty string, zero length, or zero byte. That would erase the difference
between valid data and a broken provider or invalid input.
