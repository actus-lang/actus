# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 26. Native and embedded-oriented design rules

Actus is intended to make low-level work explicit without sacrificing the
compiler's ownership model. For embedded or protocol code:

- use fixed-width integers for wire/register fields;
- use `pack` for explicit layouts and masks;
- use `Array[T, N]` for bounded static storage;
- use caller-owned `ins Buffer` storage for zero-copy mutation;
- use `abs Buffer` for read-only inspection;
- use `dat` when ownership really ends at the callee;
- isolate MMIO and foreign calls behind `unsafe extern` facades;
- document endianness, alignment, register width, and status contracts;
- keep target/ABI details in build/runtime configuration;
- test both hosted behavior and freestanding/object boundaries where relevant.

Do not claim that the current host target is already a complete bare-metal
target. ARM, Cortex-M, Cortex-A, RISC-V, STM32, RP, and other profiles need
linker layout, interrupt ABI, register/atomic capabilities, memory regions,
runtime profile, and standard-library support outside the language syntax.
