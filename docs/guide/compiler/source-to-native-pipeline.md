# Source-to-native pipeline

The lexer recognizes Actus tokens. The parser builds declarations and
expressions. The AST records source structure. Semantic analysis resolves names,
types, roles, module visibility, bounds, constants, and generic instances.

Ownership and cleanup analysis records moves, views, loans, scope exits, and
return paths. Native lowering then translates an already valid program into
machine-oriented objects and calls the linker when an executable is requested.

Do not use code generation to repair a semantic error. Do not move backend types
into source syntax or make the parser perform ownership checks.
