# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 27. What is not currently safe to assume

The following require separate evidence and must not be invented in examples:

- unbounded `for`, `while`, or general iterator syntax;
- async/await, tasks, actors, closures, lambdas, macros, or REPL;
- automatic numeric promotion or implicit casts;
- null values or nullable references;
- an unbounded heap or garbage collection;
- stable cross-platform ABI for every aggregate;
- every target-specific runtime profile;
- WASM, DWARF, incremental compilation, or parallel compilation;
- a complete `Map` standard-library API;
- a protocol or domain runtime merely because Actus can express packs, arrays,
  buffers, and fixed-width arithmetic.

If a requested feature falls into this list, report it as a language/toolchain
gap and propose the parser, AST, semantic, codegen, tooling, and test work
needed to implement it. Do not hide the gap behind an unsafe bridge or a
handwritten special case.
