# Runtime and target behavior

## Hosted provider

The current implementation calls a hosted provider for borrowed `String`
length and byte access, UTF-8 validation, buffer length, buffer byte access,
and copying a string into caller storage. The raw provider symbols are private
to `std::string`.

The public facade translates provider status into `StringError`. Applications
must not call or reproduce the raw integer status protocol.

## Freestanding boundary

The facade is hosted-only until a freestanding runtime supplies equivalent
checked operations. Supporting a new target requires the provider to preserve
the public behavior:

- validate UTF-8 before reporting successful text access;
- distinguish null, invalid encoding, bounds, and storage failures;
- respect the caller's destination capacity;
- preserve explicit buffer length, including embedded zero bytes;
- avoid hidden allocation and unbounded temporary storage.

A target is not supported merely because its compiler accepts an import of
`std::string`.

## Allocation and timing boundary

The public API does not promise a particular provider latency. It does promise
that construction uses the supplied `Buffer` and does not silently allocate a
replacement. Any platform-specific copy or validation cost belongs to the
provider and must be measured separately.

## ABI isolation

The hosted null-terminated pointer convention is an implementation detail.
Keeping it behind the facade allows the same Actus-level ownership and error
contract to be implemented by a different runtime representation later.

## Verification

The string test suite verifies ASCII length, multibyte UTF-8 byte access,
empty values, out-of-bounds behavior, caller-buffer construction, embedded
zero bytes, facade exports, and native linking without floating-point state.
