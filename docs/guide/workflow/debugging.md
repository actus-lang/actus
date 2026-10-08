# Debugging Actus programs

When a command fails, identify the first layer that rejected the input:

1. manifest and lockfile;
2. module resolution and facade visibility;
3. lexer and parser;
4. types, roles, ownership, bounds, and cleanup;
5. native lowering and object emission;
6. linker and runtime;
7. target or external hardware.

Keep the complete diagnostic, source location, command, target, and profile.
Reduce the program to the smallest reproducible case, then add a regression
test at the layer that owns the rule.
