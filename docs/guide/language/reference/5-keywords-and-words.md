# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 5. Keywords and words

The implemented vocabulary includes the following groups.

### Declarations

- `verb` declares a callable Actus function.
- `struct` declares an aggregate with named fields.
- `pack` declares a bit/byte layout with explicit storage and field offsets.
- `enum` declares a tagged sum type with variants.
- `role` declares a callable contract/interface.
- `perform` defines role methods for a concrete target type.
- `const` declares a compile-time constant.
- `import` requests a package/module facade.
- `open` exposes a declaration through a module facade.

### Ownership and resource words

- `erg` is an owned, active binding.
- `abs` is a read-only, non-owning view/borrow.
- `dat` is a terminal ownership transfer.
- `ins` is an exclusive call-scope loan.
- `ref` creates a borrow expression.
- `drop` explicitly ends ownership and cleanup for a binding.

### Control flow and expressions

- `return` exits a verb and unwinds owned resources in affected scopes.
- `loop` creates a loop body.
- `for` creates a bounded range or fixed-array index iteration.
- `repeat` is a bounded readability alias that expands to the same `for`
  contract.
- `in` separates a `for` binding from its bounded source.
- `break` exits the nearest loop.
- `continue` starts the next iteration of the nearest loop.
- `case` performs pattern matching.
- `if` is used for an `if` statement, an `if/else` expression, and case
  guards in the implemented parser.
- `else` selects the alternate branch of an if expression or statement.
- `as` performs an explicit checked primitive integer cast.
- `true`, `false`, and `_` are boolean and wildcard syntax.

### FFI and dispatch

- `unsafe` marks a foreign or otherwise unsafe boundary.
- `extern` declares a foreign ABI boundary.
- `dynamic` requests runtime role dispatch where the callable contract allows
  it.
- `meta` attaches compile-time metadata such as `test`, `target`, or
  `limitless`.

`while`, `async`, `await`, `yield`, `spawn`, and actor-related words are not
general implemented control-flow constructs. They must not be used in new
examples unless the relevant implementation has landed. Bounded `for` and its
`in` separator are implemented only in the restricted form documented in
Section 11.3. Some other words remain reserved or planned vocabulary only.
