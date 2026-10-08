# `std::string`

```actus
import std::string;
```
`std::string` provides borrowed byte and length operations plus caller-buffer-
backed UTF-8 construction. The facade does not expose the hosted null-terminated
pointer ABI.

String access is bounded and reports typed errors for invalid UTF-8, missing
termination where required, null provider state, and out-of-range access.
Owned string construction uses caller-provided storage according to the public
buffer contract.

The module is hosted-only until a freestanding target supplies the same checked
provider contract. Read-only byte inspection should use the corresponding
`abs` view; mutation uses the documented exclusive storage role.

## Public surface

The facade includes:

- `string_length` and `string_byte_at` for borrowed string access;
- `Utf8Buffer` and `utf8_from_buffer` for owned caller-buffer-backed storage;
- `utf8_from_string`, `utf8_length`, and `utf8_byte_at` for validated UTF-8
  values;
- `StringError` variants for invalid bytes, bounds, provider state, and
  storage conditions.

String and UTF-8 operations inspect through `abs`. Construction consumes the
buffer explicitly where the API takes `dat`. No public operation exposes a raw
null-terminated pointer or silently allocates replacement storage.
