# Actus Diagnostic Codes

Semantic diagnostics use stable `E####` identifiers. Messages may become
more descriptive, but a code keeps its meaning within an Alpha language
edition.

| Code range | Category |
| --- | --- |
| `E0001`–`E0002` | Lexer errors |
| `E0003`–`E0004` | Parser errors |
| `E1001`–`E1028` | Bindings, ownership, calls, types, and returns |
| `E1029`–`E1033` | Struct declarations and initialization |
| `E1034`–`E1035` | Struct mutation and field borrow conflicts |
| `E1036`–`E1038` | Method lookup and receiver validation |

Diagnostics are represented independently from terminal rendering. Every
diagnostic carries a source span, and the CLI renderer converts it to a
deterministic line-and-column message.

New diagnostics must use a new stable code, include a focused positive or
negative test, and document the code's category here.
