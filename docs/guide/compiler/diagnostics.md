# Compiler diagnostics

A diagnostic has a stable `E####` code, source span, structured category, and
human-readable message. The same diagnostic model is used by the CLI and LSP.

Read errors in this order:

1. the first source location in the current file;
2. the declaration and call contract involved;
3. the module facade and import path;
4. the ownership state before the failing operation;
5. the selected runtime and target;
6. the native or linker boundary, if semantic checks passed.

A parser error is fixed in syntax. A semantic error is fixed in types,
ownership, visibility, bounds, or contracts. A native error is fixed in lowering
or a supported target boundary. Do not hide a compiler limitation with a
foreign wrapper or duplicated implementation.
