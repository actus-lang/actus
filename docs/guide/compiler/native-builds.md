# Native builds

A native build lowers a checked Actus program to a target object or executable.
The build may include the root object, reachable module objects, runtime
bridges, and linker arguments selected by the manifest and target.

Use the strict build mode for repository validation. Native output must retain
source ownership, bounds, layout, and typed failure contracts. The native
backend does not add hidden allocation or invent missing declarations.

When a native build fails, preserve the complete compiler and linker diagnostic.
Classify whether the failure belongs to source semantics, module visibility,
native lowering, runtime linkage, target configuration, or the external linker.
