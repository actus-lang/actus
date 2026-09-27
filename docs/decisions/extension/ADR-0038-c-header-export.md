# ADR-0038: Deterministic C Header Export

- Status: Proposed
- Date: 2026-09-27
- Scope: compiler-owned C interoperability artifacts and extension action

## Context

Actus projects may need to expose stable `pack` layouts and selected
`unsafe extern "C"` entry points to existing C or C++ systems. Generating a
header in the extension from syntax would duplicate ABI knowledge and could
silently misrepresent roles, endianness, widths, struct returns, or target
calling conventions.

## Decision

The compiler owns a deterministic header-export command and library API. The
extension offers a thin “Generate C Header” action that invokes that contract
for the selected package, target, and public declarations. No header is
generated from an unvalidated or partially analyzed source tree.

The command accepts an explicit package/root, target profile, export policy,
and output path. It emits a header plus a machine-readable manifest containing
compiler version, target ABI, source hashes, exported symbols, and layout
fingerprints.

## Export policy

Only declarations explicitly public and eligible for C ABI export are
included. The exporter must reject or require an explicit policy for:

- ownership roles that cannot be represented safely in C;
- generic or role-bound declarations without a concrete specialization;
- `Result`, `Option`, arena references, and recursive references without a
  declared C representation;
- `u1..u128` and `i1..i128` widths that have no approved C mapping;
- `f32`, `f64`, `Void`, and struct returns whose target ABI is not frozen;
- `pack` layouts with unsupported backing types or endianness; and
- declarations crossing an `unsafe extern "C"` boundary without a contract.

Supported `pack` exports use fixed-width integer types and explicit masks,
offset constants, and endianness metadata rather than implementation-defined C
bitfields. The output must not suggest that a C bitfield has Actus's
deterministic layout guarantees.

## Determinism and safety

Output ordering is stable by module path and declaration span. Formatting,
include guards or module pragmas, target integer widths, and symbol names are
specified by the exporter version. ABI conflicts are hard errors, not
warnings. The exporter never executes Actus code, loads a host device, or
writes outside the requested output directory.

The extension displays the planned output path, target profile, exported
symbols, and diagnostics before generation. It uses a temporary file and
atomic replacement only after successful generation; an existing file is not
overwritten without explicit user intent.

## Verification

- [ ] Golden headers cover functions, `pack` layouts, floats, `Void`, and
      approved fixed-width integers.
- [ ] Negative tests cover generic, ownership, Result, recursive, and ABI
      ambiguity cases.
- [ ] Repeated generation with identical inputs produces byte-identical
      headers and manifests.
- [ ] Target tests verify endianness, pointer width, struct-return ABI, and
      unsupported host services.
- [ ] Extension tests verify preview, cancellation, path validation, and
      atomic output behavior.

## Non-goals

This ADR does not promise C++ bindings, C bitfield compatibility, automatic
FFI safety, or a replacement for a target vendor SDK. It does not make every
Actus public declaration exportable.
