# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 1. The most important rule for an agent

Before changing Actus code, determine which of the following is true:

1. The syntax or behavior is implemented and tested.
2. The behavior is implemented but has a known target, runtime, ABI, or
   standard-library boundary.
3. The behavior is described as a design but is not implemented.
4. The request requires a new language or compiler capability.

Only the first two categories may be used as existing Actus behavior. Category
three is a design reference, not an API. Category four requires a design and
tests before implementation. Never copy a future keyword, a future target
profile, or a future library contract into an example and present it as
working code.

When uncertain, inspect the lexer, AST, parser, semantic analyzer, native
lowering, tests, and examples in that order. A passing parser test alone does
not prove semantic validity or native execution.
