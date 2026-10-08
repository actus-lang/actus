# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 21. Build manifests and runtime profiles

The root manifest has this general shape:

```toml
[package]
name = "my_program"
version = "0.1.0"
edition = "alpha"
entry = "main"

[build]
target = "host"
profile = "debug"
runtime = "std"
native_module = "actus"
position_independent = true

[dependencies]
```

Use `runtime = "core"` or omit the standard-library opt-in when the package
must remain dependency-free. Use `runtime = "std"` for compiler-owned hosted
I/O, filesystem, and path modules.

Dependencies use manifest-relative package paths:

```toml
[dependencies]
io = { path = "library/std" }
fs = { path = "library/std" }
path = { path = "library/std" }
```

Run `actus lock` after changing dependencies. `actus lock --check` verifies
that `Actus.lock` agrees with the manifest and package checksums.

The current hosted target is `host`. Target profiles are compiler/toolchain
configuration, not hard-coded language types. Future ARM Cortex, RISC-V,
STM32, RP, WASM, or other profiles must be added through target manifests and
backend/runtime contracts rather than by changing the meaning of `u32`,
`Buffer`, or `pack` in source.
