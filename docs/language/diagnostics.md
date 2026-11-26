# Actus Diagnostic Codes

Semantic diagnostics use stable `E####` identifiers. Messages may become
more descriptive, but a code keeps its meaning within an Alpha language
edition.

The catalog in [`diagnostics/`](diagnostics/) holds one entry per code, named
after it (`diagnostics/E1140.md`): what the code means, a program that
triggers it and the fix. The compiler's test suite builds every example and
checks the catalog against the registry of codes, so an entry cannot drift from
the compiler. Codes without an entry are listed in `diagnostics/missing.txt`
until their entry is written.

Every diagnostic prints in one format, whatever finds it. A diagnostic that
points into a file is `<path>: error[E####] at line:column: message`. A
manifest error names the manifest (`Actus.toml`) and the line and column of the
key or value it is about, and `<path>: error[E####]: message` when the manifest
cannot say where. An error of the toolchain or the environment names no file:
`error[E####]: message`. The code belongs to the error, not to the mode, so a
manifest error has the same code with and without `--strict`. A finding that
fails the run is an `error`; strict mode makes every finding an error
(ADR-0041), and a `warning` never changes the exit status.

| Code range | Category |
| --- | --- |
| `E0001`–`E0002`, `E0024`–`E0027` | Lexer errors: unexpected character, unterminated string, unterminated doc string, invalid integer type, invalid hexadecimal literal, invalid float type |
| `E0003`–`E0004` | Parser token and end-of-input errors |
| `E0005`–`E0009` | Parser declaration, keyword, and metadata errors |
| `E0010`–`E0023` | Parser target, metadata scope, contract, effect, capability, representation, and limit errors |
| `E1001`–`E1028` | Bindings, ownership, calls, types, and returns |
| `E1029`–`E1033` | Struct declarations and initialization |
| `E1034`–`E1035` | Struct mutation and field borrow conflicts |
| `E1094`–`E1096` | Struct field-role, frozen-field, and self-assignment rules |
| `E1036`–`E1038` | Method lookup and receiver validation |
| `E1040`–`E1079` | Enum patterns, roles, generics, packs, and arenas |
| `E1081`–`E1082` | Duplicate packed layouts and escaping loans |
| `E1083`–`E1099` | Bounded arrays, arenas, dynamic roles, and compile-time expressions |
| `E1100`–`E1108` | Module paths, facades, sibling visibility, and module I/O |
| `E1109` | Private module declaration or public-signature visibility violation; reached unqualified or qualified (`module::name`); the help names the cause: mark the declaration `open`, or, when it is already `open`, change `open unit x;` to `open x;` or add `open x;` in the named facade |
| `E1119` | Reference outside the hierarchical scope of its directory module |
| `E1137` | Facade opens one name both publicly and for its unit |
| `E1139` | `print(int:)`, `eprint(int:)`, or a `format:` print with `int:`, given an integer wider than 64 bits |
| `E1138` | Trapping `+`, `-`, `*` or `/` on integer literals overflows its type (`/` only for the minimum value divided by `-1`) |
| `E1140` | Arithmetic, bitwise or comparison operator given operands of different integer types |
| `E1141` | Call to a verb that returns nothing used where a value is needed (binding, assignment, argument, operand, condition, `return`, conditional value or field initializer); such a call is only valid as a statement of its own |
| `E1142` | Assignment to a constant (`const`); constants are fixed at compile time, use an `erg` binding for a value that changes |
| `E1143` | Test layout violation (ADR-0121): a `tests` directory with a facade, a `src/tests` directory, a subdirectory inside a `tests` directory, or an `.act` file in a subdirectory of the project `tests/` |
| `E1144` | `meta test` verb in a production source file; move it to a file of the module's `tests` directory |
| `E1145` | Move of a field out of a `zero_on_drop` owner; the owner is zeroed as a whole, so a field cannot leave it |
| `E1090` | Cast between types that are not both integer types |
| `E1091` | Integer literal in a cast is outside the range of the target type |
| `E1116`–`E1118` | Dynamic role dispatch and constant declaration validation |
| `E1110` | Deterministic native symbol or ABI collision across module implementations |
| `E1104` | Unknown sibling referenced by a module facade |
| `E1105` | Module source read failure or defensive module-source fallback |
| `E1106` | Duplicate declaration aggregated across module sources |
| `E1800`–`E1809` | Strict configuration and policy |
| `E1810`–`E1819` | Strict frontend validation |
| `E1820`–`E1829` | Strict semantic validation |
| `E1830`–`E1839` | Strict architecture validation |
| `E1840`–`E1849` | Strict documentation validation |
| `E1850`–`E1859` | Strict source-limit validation |
| `E1860`–`E1879` | Strict execution |
| `E1880`–`E1889` | Bundled toolchain and linker validation |
| `E1890`–`E1899` | Reserved expansion |

| Code | Description |
| --- | --- |
| `E1880` | Required bundled linker or compiler-rt component is missing |
| `E1881` | Selected linker identity or target does not match the bundled contract |
| `E1882` | Strict mode rejects a manually selected freestanding linker |
| `E1801` | Package manifest name is not a valid or unique identity namespace |

Diagnostics are represented independently from terminal rendering. Every
diagnostic carries a source span, and the CLI renderer converts it to a
deterministic line-and-column message.

`E1110` is emitted when two module implementations claim one native symbol
with incompatible ABI contracts, such as different ownership roles. The
diagnostic includes the native symbol and both module namespaces. A generic
type that renders with the same display text at two compiler boundaries is not
reported as an ABI collision unless the resolved native contracts actually
differ.

`E1119` is emitted when a spelling in a directory module names a declaration
of its unit that is not in that module's scope (ADR-0110): a declaration of a
sibling or cousin directory that the module does not import, a private
declaration of a child directory, or a name brought in only by an import of
another module of the unit. It is also emitted for a name that is reached
through an import of another unit's child module but is not exported by that
child. Imports are not inherited by child modules. The
diagnostic names the referencing module and, for an out-of-scope directory,
the `import { open <module> }` that brings the declaration into scope.

`E1137` is emitted when one facade opens the same sibling file or child
directory both with `open x;` and with `open unit x;` (ADR-0111). Keep one of
the two openings.

`E1143` is emitted for a test file or directory that is in the wrong place. Module
tests live in `src/<module path>/tests/*.act`, directly in that directory;
project tests live in `tests/*.act`, directly in that directory. A `tests`
directory is never a module, so it must not contain a facade `tests.act`, and
the root unit has no `src/tests` directory. The message names the file and,
for a file of the old mirror layout (`tests/<path>.act`), the module `tests`
directory it belongs in.

`E1144` is emitted when a `meta test` verb is declared in a production source
file, that is, anywhere under the source root outside a `tests` directory. The
message names the module's `tests` directory; the root unit has none, so its
logic moves into a module or is tested from `tests/` through what it opens.

`E1142` is emitted when an assignment, compound assignment, field assignment
or indexed assignment names a `const`. A constant is fixed at compile time and
is not a binding; declare an `erg` binding for a value that changes.

Several existing codes name their problem precisely rather than generically:

- `E1063` reports `` `if` condition must be `Bool`, found `T` `` for an `if` or
  `else if` condition, and `case guard must return `Bool`` for a case guard.
- `E1011` reports `cannot move frozen binding` when a binding with a live `abs`
  view is moved, and `cannot assign to frozen binding` when it is assigned to.
- `E1025` names `integer` as the expected operand type when an arithmetic,
  bitwise or shift operator is given a floating-point or `Bool` left operand
  (Actus arithmetic is integer-only), and never reports the same type as both
  expected and found.
- `E0009` is also reported for `while`, which is not an Actus keyword; the
  message names `loop`, `for` and `repeat`.

New diagnostics must use a new stable code, include a focused positive or
negative test, and document the code's category here. Strict-conformance codes
must remain inside the reserved `E1800`–`E1899` range and use the category
sub-range assigned above.

The documentation validator reports `E1840` for a missing or empty block,
`E1841` for a missing contract section, `E1842` for an incomplete or
signature-restating summary, `E1844` for an ownership contradiction, and
`E1845` for a result guarantee that misrepresents implementation behavior.
Strict mode (`actus check`, `build` and `test` with `--strict`, for every source
file of the package) currently enforces only `E1840`, `E1844` and `E1845`. The
content checks `E1841` and `E1842` are keyword based and not reliable yet, so
they are not reported by the CLI; they come with phase 56. `E1843` is not
assigned and is not reported. Without `--strict` documentation is never checked.

The source-limit validator measures code-bearing lines. Blank lines, Rust-style
`//` and `/* ... */` comments, and Actus `#` comments do not count; inline
comments are ignored after the code on their line has been counted. Actus
triple-quoted `""" ... """` documentation blocks remain counted because they
are language constructs. `E1850` is reported when a file exceeds the preferred
500 code-line size or `E1853` when a function exceeds the preferred 50
code-line size. These two are warnings in the report of `actus conformance`,
which does not fail the run, and errors under `--strict`.
`E1851` and `E1854` are strict split-required failures at 600 file lines and
60 function lines. `E1852` and `E1855` reject files above 800 lines and
functions above 80 lines respectively.

## Retired codes

A code the compiler can no longer emit is retired (ADR-0123). Its number stays
in the registry's `RETIRED` list and is never given to another diagnostic; it
has no catalog entry. The table names the code that reports the case now.

| Code | Former meaning | Reported now by |
| --- | --- | --- |
| `E1039` | Duplicate enum declaration found by semantic analysis | `E0005` for one file, `E1106` for two files of one module, `E1110` for two modules |
| `E1049` | Type of a pattern binding differs from the expected type | `E1048`; a binding takes the type of its payload, so only the pattern as a whole can differ from the subject |
| `E1064` | Binding used while an exclusive loan suspends it | `E1065` for a second use of the owner in the call that borrows it |
| `E1075` | Pack field with an access role other than `erg` or `abs` | `E0003`, from the parser |
| `E1080` | Malformed internal type name of a `for` range binding | `E1025`, the binding is not an integer type |
| `E1107` | Lexical failure with no error at a module boundary | The lexical error itself (`E0001`, `E0002`, `E0024`–`E0027`), with the path of the module file |
| `E1111` | One module owning two object units | None; a module that is imported more than once is loaded once |
