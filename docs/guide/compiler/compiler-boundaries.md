# Compiler boundaries

Some behavior is implemented in the language, some in the standard library,
and some in the compiler or target runtime. Keep those responsibilities
visible.

Do not assume unsupported syntax such as unbounded loops, async execution,
closures, macros, an unbounded heap, or a target-specific runtime merely
because a design document mentions it. Do not use a future keyword in a current
example.

When a required capability is missing, record the exact diagnostic and identify
the parser, AST, semantic, codegen, runtime, tooling, and test work required.
The source program should remain an honest representation of the intended
contract while the capability is being designed.
