# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 24. Formatter rules

The formatter is a source-preserving language tool, not a minifier. It may
normalize indentation and spacing, but it must preserve:

- triple-quoted Actus documentation strings;
- imports and module facade declarations;
- declaration order and semantic boundaries;
- comments/docstrings attached to the declaration they describe;
- all tokens required for a reparsable program.

Formatting must be idempotent: formatting an already formatted file produces
the same file. Always run `actus fmt --check` after formatting and then run
`actus check` on the result. If formatting moves or deletes documentation,
imports, or declarations, treat it as a compiler bug and add a regression
test rather than accepting the output.
