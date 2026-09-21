# ADR 0001: C FFI Boundary

## Status

Accepted for the Alpha compiler.

## Decision

Actus treats every external C declaration as an unsafe boundary. Source code
must write `unsafe extern "C" verb ...;` before importing a foreign symbol.
The marker makes unchecked foreign behavior visible at the call boundary and
prevents an ordinary declaration from silently acquiring C ABI semantics.

The initial C ABI supports `Int` as `int32_t` and `Buffer` as an opaque
pointer. `abs` and `dat` roles are valid only for opaque resource pointers:

- `erg Buffer` is an exclusive pointer passed to the foreign function.
- `abs Buffer` is a shared read-only borrow valid for the call duration.
- `dat Buffer` transfers ownership to the foreign function.
- `Int` is a value and cannot be marked `abs` or `dat`.

Opaque pointer returns transfer ownership to the Actus caller. Primitive
returns are values. String, collection, raw-pointer, and foreign-resource
layouts without an explicit contract remain unsupported at this boundary.

The compiler validates these rules before native emission. The linker only
handles library selection and does not infer ownership or repair an unsafe
foreign declaration.

## Consequences

Foreign functions remain usable without embedding C source in Actus, while
ownership and borrowing remain explicit at the language boundary. Additional
ABI types require a dedicated representation and ownership contract before
they can be enabled.
