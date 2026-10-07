# Freestanding Provider Target Build Evidence

## Scope

This record covers the feature-gated freestanding Region provider shim. It
records compilation and object-symbol evidence for the installed Cortex-M4F
profile. It does not claim that the shim has been flashed, linked with a
board's allocator, or measured on physical hardware.

## Environment

- Repository: Actus compiler
- Branch: `phase-36-native-enum-discriminant`
- Target: `thumbv7em-none-eabihf`
- Toolchain: stable `rustc 1.97.1`
- Profile: Cargo `dev`
- Provider window: 8 slots, 1024 bytes per resident/published window pair

## Commands

```text
cargo check --manifest-path runtime/freestanding/provider_core/Cargo.toml \
  --target thumbv7em-none-eabihf --features target-abi

cargo build --manifest-path runtime/freestanding/provider_core/Cargo.toml \
  --target thumbv7em-none-eabihf --features target-abi --lib

cargo rustc --manifest-path runtime/freestanding/provider_core/Cargo.toml \
  --target thumbv7em-none-eabihf --features target-abi --lib -- --emit=obj
```

## Results

- `cargo check`: passed.
- `cargo build`: passed.
- Emitted object: ARM 32-bit ELF, EABI5, relocatable.
- Largest emitted debug object observed: 215,652 bytes.
- Exported symbols confirmed with `nm -g --defined-only`:

```text
actus_region_cancel
actus_region_close
actus_region_drop
actus_region_open
actus_region_publish
actus_region_read
actus_region_write
```

## Boundary

The object still requires target-provided `actus_buffer_drop` and
`actus_enum_allocate` implementations for a final executable link. No MCU
resident-memory, interrupt-safety, cycle-count, or power measurement is
claimed by this record.
