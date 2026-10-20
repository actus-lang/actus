# ADR-0083: Bounded Charmap Codecs

- Status: Proposed
- Date: 2026-10-20
- Scope: Actus standard library text encoding

## Context

Actus applications such as Twin-e need a compact, deterministic representation
for a restricted alphabet while retaining UTF-8 at external boundaries. A
language-specific byte map must not change the existing `String` ABI, hide
allocation, or be confused with ASCII or Unicode.

## Decision

Add a hosted `std::charmap` module with explicit bounded codecs.

The first codec is `std::charmap::georgian`. It maps the 33 modern Georgian
letters to a reserved one-byte range and preserves ASCII bytes unchanged.
The representation is an internal codec, not ASCII and not a replacement for
UTF-8. Callers must explicitly request encoding or decoding.

The public boundary is caller-owned and buffer-backed:

- encoding consumes a UTF-8 source buffer and writes one-byte mapped output;
- decoding consumes mapped bytes and writes UTF-8 output;
- capacity exhaustion, malformed UTF-8, unmapped bytes, invalid storage, and
  unavailable providers are typed failures;
- failed operations must not publish a partial output result;
- no floating-point state, hidden allocation, or runtime pointer is exposed.

The module is organized as a canonical facade, shared error contract, and
language-specific codec. Additional alphabets may be added as sibling codecs
without changing the public `std::charmap` boundary.

## Alternatives considered

### Replace the standard `String` representation

Rejected. Existing `String` and `Utf8Buffer` contracts remain unchanged.

### Call the mapped representation ASCII

Rejected. Values above 127 are private codec bytes and require an explicit
codec identity to be interpreted correctly.

### Put the map in the compiler or tokenizer

Rejected. Encoding policy belongs to the standard-library/application boundary,
while tokenization remains an application concern.

### Use an unbounded dynamic map

Rejected. The first implementation must remain deterministic and bounded for
hosted and future embedded consumers.

## Consequences

Positive consequences:

- Twin-e can use a compact Georgian representation without changing Actus's
  existing string ABI.
- UTF-8 interoperability remains explicit and lossless.
- Codec errors are visible to callers and cannot silently corrupt text.
- Future codecs share one stable module boundary.

Costs:

- Callers must provide output storage and select the codec explicitly.
- The mapped bytes are meaningful only with their codec identity.
- Full Unicode normalization and locale-aware text processing remain outside
  this module.

## Verification requirements

Acceptance requires semantic facade visibility, successful encode/decode
round-trips, ASCII preservation, malformed and unmapped input rejection,
capacity failure without partial publication, formatting, strict checks, and
native execution on the supported backends.
