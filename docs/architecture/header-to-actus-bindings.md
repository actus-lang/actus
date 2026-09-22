# C Header-to-Actus Binding Workflow

Actus does not treat C source generation as a compiler phase. Header binding
is an explicit FFI preparation workflow that produces reviewed Actus
declarations and an Arca build configuration.

## Workflow

1. Select the target headers, include paths, preprocessor defines, and target
   profile in a versioned binding manifest.
2. Run a header inspection or generation tool outside the semantic compiler.
3. Review the generated declarations and retain only the symbols required by
   the package.
4. Express each imported function as an explicit `unsafe extern "C" verb`.
5. Record pointer ownership, nullability, layout, calling convention, and
   library linkage in the binding manifest.
6. Compile the declarations through Actus semantic checks and link them using
   the existing Arca native library settings.
7. Test the binding with a target-specific link smoke test and an ABI/layout
   regression test.

## Boundaries

Generated declarations are source artifacts and must be reviewed like hand-
written FFI code. The binding workflow must not infer Actus ownership from C
function names. Ownership and lifetime at the boundary must be explicit in
the declaration and manifest.

The compiler validates Actus declarations and ABI contracts; it does not
parse arbitrary C headers, choose platform libraries, or provide hardware
drivers. A future `arca bind` command may automate this workflow without
moving those responsibilities into `actusc`.
