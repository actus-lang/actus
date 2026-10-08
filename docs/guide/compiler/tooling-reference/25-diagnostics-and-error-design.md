# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 25. Diagnostics and error design

Diagnostics have stable codes, source spans, human-readable messages, and a
structured representation independent of terminal rendering. When adding a
new diagnostic:

1. assign a stable `E####` code;
2. identify the exact source span;
3. explain the violated language contract;
4. provide a correction when it is unambiguous;
5. add a positive/negative regression test;
6. verify CLI, LSP, and formatted rendering;
7. ensure malformed input does not panic or terminate the compiler.

A parser may reject malformed syntax. Semantic analysis must reject invalid
types, ownership, module visibility, bounds, constants, and ABI contracts.
Code generation must report an internal boundary error rather than inventing a
semantic repair.
