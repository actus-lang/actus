# Actus Diagnostic Codes

Semantic diagnostics use stable `E####` identifiers. Messages may become
more descriptive, but a code keeps its meaning within an Alpha language
edition.

| Code range | Category |
| --- | --- |
| `E0001`–`E0002` | Lexer errors |
| `E0003`–`E0004` | Parser token and end-of-input errors |
| `E0005`–`E0009` | Parser declaration, keyword, and metadata errors |
| `E1001`–`E1028` | Bindings, ownership, calls, types, and returns |
| `E1029`–`E1033` | Struct declarations and initialization |
| `E1034`–`E1035` | Struct mutation and field borrow conflicts |
| `E1036`–`E1038` | Method lookup and receiver validation |
| `E1040`–`E1079` | Enum patterns, roles, generics, packs, and arenas |
| `E1080`–`E1082` | Malformed generic names, duplicate packed layouts, and escaping loans |
| `E1100`–`E1103` | Module path validation, facade discovery, and module I/O |
| `E1104` | Unknown sibling referenced by a module facade |
| `E1105` | Module source read failure or defensive module-source fallback |
| `E1106` | Duplicate declaration aggregated across module sources |
| `E1107` | Invalid empty lexical failure reported by a module boundary |
| `E1800`–`E1809` | Strict configuration and policy |
| `E1810`–`E1819` | Strict frontend validation |
| `E1820`–`E1829` | Strict semantic validation |
| `E1830`–`E1839` | Strict architecture validation |
| `E1840`–`E1849` | Strict documentation validation |
| `E1850`–`E1859` | Strict source-limit validation |
| `E1860`–`E1899` | Strict execution and reserved expansion |

Diagnostics are represented independently from terminal rendering. Every
diagnostic carries a source span, and the CLI renderer converts it to a
deterministic line-and-column message.

New diagnostics must use a new stable code, include a focused positive or
negative test, and document the code's category here. Strict-conformance codes
must remain inside the reserved `E1800`–`E1899` range and use the category
sub-range assigned above.
