# Freestanding Target Support Link Evidence

## Scope

This record proves that the freestanding Region provider can be linked against
separate target-owned Buffer and Result support symbols. The support package is
`no_std`, fixed-capacity, and independent from the provider implementation.

This is ARM object-link evidence. It is separate from the STM32F411 runtime
execution evidence in `phase-37-f411-hardware-2026-10-07.md`.

## Inputs

- Provider package: `runtime/freestanding/provider_core`
- Target support package: `runtime/freestanding/target_support`
- Target: `thumbv7em-none-eabihf`
- Provider feature: `target-abi`
- Result support: one fixed 128-byte target-owned slab
- Allocator: no general allocator and no libc

The target support package exports:

```text
actus_buffer_drop
actus_enum_allocate
actus_enum_drop
```

The provider object exports:

```text
actus_region_open
actus_region_read
actus_region_write
actus_region_publish
actus_region_cancel
actus_region_close
actus_region_drop
```

## Commands

```text
cargo rustc --manifest-path runtime/freestanding/provider_core/Cargo.toml \
  --target thumbv7em-none-eabihf \
  --features target-abi \
  --lib -- --emit=obj

cargo rustc --manifest-path runtime/freestanding/target_support/Cargo.toml \
  --target thumbv7em-none-eabihf \
  --lib -- --emit=obj

rust-lld -flavor gnu -r provider.o target_support.o \
  -o /tmp/actus-freestanding-provider-linked.o
```

The standalone `ld.lld` command was unavailable in the host environment. The
same LLVM linker shipped with the Rust toolchain (`rust-lld`) performed the
relocatable link.

## Result

The linked output was:

```text
ELF 32-bit LSB relocatable, ARM, EABI5 version 1 (SYSV), with debug_info, not stripped
```

Exported symbols in the linked object:

```text
actus_buffer_drop
actus_enum_allocate
actus_enum_drop
actus_region_cancel
actus_region_close
actus_region_drop
actus_region_open
actus_region_publish
actus_region_read
actus_region_write
```

The final unresolved-symbol check returned:

```text
none
```

## Acceptance boundary

- [x] Provider and target support are separate packages.
- [x] Target support uses fixed storage and `no_std`.
- [x] All seven Region symbols are present.
- [x] Buffer release and Result allocation/drop symbols resolve from the
      separate target support object.
- [x] The linked ARM object has no unresolved `actus_*` symbols.
- [ ] This object is not yet a complete Actus-generated application image.
- [ ] This evidence does not measure target timing, power, or interrupt safety.
