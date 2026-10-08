# Actus terminology

Use these meanings throughout the handbook.

| Term | Meaning |
|---|---|
| binding | A named value together with its type, ownership role, and current state. |
| owner | The binding responsible for a resource and its cleanup. |
| view | A non-owning way to inspect a value for a limited scope. |
| loan | An exclusive, call-scoped mutable access to an existing owner. |
| role | The ownership meaning written as `erg`, `abs`, `dat`, or `ins`. |
| verb | An Actus function declaration. |
| facade | The canonical module file that defines the public boundary of a module. |
| sibling | An implementation file inside the same directory module as its facade. |
| runtime profile | The standard-library and execution boundary selected for a package. |
| target | The platform and ABI contract selected for checking or native output. |
| hosted | A build that uses the host runtime services available to an application. |
| freestanding | A build whose runtime services are supplied through an explicit target contract. |
| native object | A compiled object file produced before final linking. |
| executable | A linked native program with an entry point. |
| typed failure | A `Result` or other declared return value that represents an operation failure. |
| bounded storage | Storage with a declared fixed capacity and checked access. |
| source contract | The syntax, type, ownership, visibility, and cleanup rules checked from Actus source. |
| implementation boundary | A point where behavior depends on the compiler, runtime, target, ABI, or standard library. |

Use the same term for the same concept. Introduce a new term only when the
existing vocabulary cannot describe the concept precisely.

## Notation

- Actus keywords and identifiers appear in backticks.
- File paths use repository-relative links.
- Commands appear in fenced `sh` blocks.
- Actus examples use fenced `actus` blocks.
- Compiler diagnostic codes use their stable form, such as `E1016`.
- A public API description names its import, arguments, ownership roles,
  return value, failure values, cleanup, and allocation behavior when those
  details apply.
