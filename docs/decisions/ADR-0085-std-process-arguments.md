# ADR-0085: Hosted `std::process` Argument API

- Status: Accepted
- Date: 2026-10-21
- Scope: Hosted standard library and production application entry points

## Decision

Actus will expose process arguments through the hosted standard library module
`std::process`. Argument handling is not a language-core feature and does not
change the `main()` entry-point signature.

The public API is a bounded cursor over caller-owned buffers:

```act
open verb arguments() -> Result[Arguments, ProcessError];
open verb count(abs arguments: Arguments) -> Int;
open verb next_arg(
    ins arguments: Arguments,
    ins buffer: Buffer
) -> Result[Option[Int], ProcessError];
```

`arguments()` creates a cursor over the host process argument vector. Each
`next_arg` call copies one argument into the supplied buffer and advances the
cursor. `Ok(Some(byte_count))` reports one argument, including an empty
argument; `Ok(None)` reports the end of the argument vector. The public API
does not expose native pointers, Rust strings, or an unbounded collection.

The first yielded argument is the host executable path (`argv[0]`), followed by
the user-supplied arguments. `count()` returns the validated total count,
including `argv[0]`. Every `next_arg` invocation overwrites the supplied
caller-owned buffer from offset `0`; it does not append to the previous result.

The initial hosted limits are 64 arguments and 4096 UTF-8 bytes per argument.
The destination buffer must already have enough capacity. Invalid indexes,
invalid argument encoding, too many arguments, and insufficient destination
capacity are typed `ProcessError` outcomes and leave caller-owned state
unchanged where applicable.

The raw runtime bridges remain private implementation details:

```text
std::process facade
        |
arguments.act / error.act
        |
unsafe extern "C" runtime bridges
        |
host process argument provider
```

The API is hosted-only. Freestanding and embedded targets do not acquire a
process model or host command-line state through this decision. Embedded
configuration must use a target-specific boot/configuration provider, Flash,
UART/USB, or `std::wire` boundary. Twin-e core code must not depend on
`std::process`.

## Rejected alternatives

1. Adding `argv` parameters to `main()`: rejected because it changes the
   language entry contract and creates a host-only core-language special case.
2. Returning a dynamic argument array: rejected because it violates bounded
   storage and hides ownership/allocation behavior.
3. Reading arguments through an undocumented environment or hardcoded path:
   rejected because production runners need an explicit, testable contract.
4. Exposing raw C-ABI functions to application code: rejected because the
   standard-library facade must own error mapping and buffer contracts.

## Verification

- The facade resolves through `library/std/src/lib.act` on hosted targets.
- Native tests cover argument count, ordered cursor reads, empty arguments,
  insufficient buffers, invalid indexes, and bounded overflow behavior.
- Cranelift and LLVM compile the same fixture and link the same runtime symbols.
- A freestanding compile proves that no process runtime symbols are required
  by the target-neutral core.
- The production runner records the compiler/runtime revision used to build
  its executable.
