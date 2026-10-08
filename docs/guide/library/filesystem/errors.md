# Filesystem errors

Filesystem operations use `IoError` from `std::io` so applications can handle
I/O failures consistently across console, file, and path APIs.

## `Failed`

The host rejected the operation or the provider could not complete it. This
covers missing paths, permissions, invalid host state, failed close, failed
seek, rejected directory operations, and failed publication steps.

## `EndOfStream`

A read or write reached the provider's end-of-stream status. A read caller may
use the successful byte count it already received; it must not silently treat
an end status as arbitrary data.

## `InvalidInput`

The operation received an invalid input under the shared I/O contract. Path
construction and option selection should normally reject invalid values before
filesystem execution, but this result remains part of the public error domain.

## `InvalidData`

The bytes were not valid for the requested representation. `read_to_string`
uses this error for invalid UTF-8 and does not return the malformed buffer as
text.

## Error ownership

The facade cleans temporary file and buffer resources before returning an
error. The caller keeps ownership of borrowed paths and write buffers. The
caller owns successful returned buffers and files and must handle them as moved
values.

## Preserve distinctions

Do not map every `IoError` to an empty buffer, zero byte count, or a generic
success. `Failed`, `EndOfStream`, `InvalidInput`, and `InvalidData` represent
different recovery decisions.
