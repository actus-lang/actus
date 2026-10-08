# Comments and documentation

Actus has short implementation comments and declaration documentation.

## Short comments

Use `#` for an implementation comment:

```actus
# The length field excludes the checksum trailer.
erg payload_length: u16 = frame.payload_length;
```

`//`, `///`, and foreign-language comment syntax are not Actus comments.

## Documentation blocks

Use `""" ... """` for documentation attached to a declaration:

```actus
"""
Reads one bounded frame from the caller-owned input storage.

The input is inspected through abs. The returned Result reports either the
parsed frame or a typed decoding error. The operation does not allocate.
"""
verb read_frame(abs input: Buffer) -> Result[Frame, DecodeError] {
    ...
}
```

Public types, packs, enums, external declarations, constants, and verbs require
documentation. The block must describe implemented behavior and the source
contract; it must not describe an unimplemented future API as usable code.

## Writing documentation

Explain the purpose first, then the inputs and outputs. State ownership roles
and cleanup when they affect callers. State allocation and I/O behavior when
the operation has a boundary with a runtime or filesystem.

Keep examples close to the declaration they explain. Use links to focused
handbook pages for long explanations instead of copying the same rule into
every page.
