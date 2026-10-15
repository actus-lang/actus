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

## Backend and target selection

The backend is selected by the manifest or equivalent CLI configuration.
Cranelift remains the default when its Cargo feature is enabled; LLVM requires
the `llvm` feature. Backend selection happens after semantic analysis and
generic specialization, so both backends consume the same source contract.

```toml
[build]
backend = "llvm"
target = "x86_64-unknown-linux-gnu"
runtime = "std"
verify_no_float_ir = true
```

For a freestanding object, declare the entry contract explicitly:

```toml
[build]
backend = "llvm"
target = "x86_64-unknown-uefi"
entry_contract = "freestanding"
verify_no_float_ir = true
```

Hosted and freestanding builds must preserve the same entry ABI, ownership
cleanup, runtime bridge, and integer-only guarantees. Freestanding builds do
not acquire hosted services implicitly. Large aggregates at or above
`LARGE_AGGREGATE_THRESHOLD` are rejected at the shared lowering boundary on
freestanding targets; hosted storage follows the explicit policy in ADR-0081.

For backend parity evidence, compare hosted executable exit status and compare
freestanding object entry exports plus referenced runtime bridge relocations.
Raw undefined declarations are insufficient evidence because a backend may
declare an unused cleanup bridge eagerly.

## Failure classification

- An operation/source-span/backend/target diagnostic is a native lowering
  failure and must not fall back to the other backend.
- A missing referenced runtime bridge is a runtime linkage failure.
- A mismatch in object entry exports or referenced bridge relocations is a
  backend ABI/parity failure.
- A large-aggregate rejection on a freestanding target is an intentional
  target-policy semantic error, not a linker failure.
