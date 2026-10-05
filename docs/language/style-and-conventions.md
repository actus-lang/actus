# Actus Language Style and Conventions

Version: 0.1

Status: Accepted language design

This document defines the canonical style, visibility, import, naming, and
documentation conventions for Actus and the Actus ecosystem.

The compiler is still under active development. Rules marked as planned are
part of the accepted design but are not necessarily implemented in the alpha
compiler yet.

## 1. Comments and Documentation

The accepted ordinary comment syntax is:

```act
# configure the hardware register base
erg base_addr = 0x40011000;
```

Documentation uses an explicitly closed triple-quoted block:

```act
"""
This describes the zero-copy transfer protocol.
"""
open verb transfer(abs payload: Buffer) {
    ...
}
```

The documentation block directly before an `open` declaration documents that
public API declaration:

```act
"""
Configure the device interrupt line.
The declaration is visible to importing modules.
"""
open verb configure_interrupt() {
    ...
}
```

The compiler ignores ordinary `#` comments while preserving documentation
blocks as structured `DocString` tokens. Formatter, LSP, and documentation
tools must preserve their source spans and text.

### 1.1 Structured verb contracts

Phase 33.6 defines an optional structured form for a verb's leading
documentation block. The first non-empty line must be exactly `contract:`. The
accepted sections are `purpose`, `inputs`, `outputs`, `ownership`,
`invariants`, `errors`, `side_effects`, and `abi`:

```act
"""
contract:
purpose:
    Read one frame.
inputs:
    source: immutable frame bytes.
outputs:
    Returns a frame or a typed error.
ownership:
    The input remains caller-owned.
"""
open verb read_frame(abs source: Buffer) -> Result[Frame, DecodeError] {
    ...
}
```

These sections are documentation metadata. They do not create runtime
preconditions, alter ownership, or change native code generation. The full
contract and rejection rules are defined in ADR-0065. This syntax is accepted
design while Gate 33.6 implementation is in progress.

## 2. Return Types

Return types use the existing arrow syntax:

```act
verb calculate(erg x: Int) -> Int {
    return x + 1;
}
```

An owned value return is the default. No additional `dat` annotation is used
on return types. Borrowed values must not escape their lexical scope.

## 3. Imports and Module Namespaces

Module paths use `::` separators. An import creates a module namespace; it
does not place every declaration directly into the current scope.

```act
import driver::gpio;
import driver::uart as serial;

gpio::set_high();
serial::write(packet);
```

The final path segment is the default namespace name. `as` assigns an explicit
namespace alias.

Grouped imports are syntax sugar for multiple single imports:

```act
import {
    driver::gpio,
    driver::uart as serial,
};
```

The following rules apply:

- grouped and single imports have identical semantic behavior;
- duplicate namespace names and alias collisions are compile errors;
- module resolution is controlled by `Actus.toml` and the package graph;
- imports must not depend on arbitrary unresolved filesystem paths;
- circular imports must produce a deterministic diagnostic;
- only `open` declarations are visible across module boundaries.

The module resolver and grouped import syntax are planned compiler features.

### 3.1. Directory Modules and Internal Scoping

A directory forms one cohesive module namespace and one compilation unit. A
multi-file module must contain a root entry file whose name exactly matches
the directory:

```text
src/
└── driver/
    └── gpio/
        ├── gpio.act       // canonical facade for driver::gpio
        ├── registers.act  // internal layout declarations
        ├── roles.act      // internal role declarations
        └── ops.act        // internal verb implementations
```

The root entry file is the module's sole external facade. Only declarations
marked `open` in `gpio/gpio.act` are visible to an external importer. An
`open` declaration in a sibling file remains module-internal unless the
facade explicitly exposes the corresponding API.

The facade exposes a sibling file with an extension-free declaration:

```act
open registers;
open ops;
```

The name resolves only to a direct `<name>.act` sibling. The sibling's own
`open` declarations are re-exported through the facade; its closed
declarations remain module-private. A sibling that is not named by the facade
does not contribute any external exports, even when it contains `open`
declarations. The facade contains no implementation declarations; it is only
a gateway that lists sibling files whose internal `open` declarations become
external exports.

All `.act` files directly inside the same module directory share one internal
scope. Sibling declarations can refer to one another without `import`
statements, relative paths, or re-export boilerplate. Types, roles, enums,
and helper verbs therefore remain local to the directory compilation unit.

The compiler must discover sibling files deterministically and reject
duplicate declarations in the shared namespace. File-system enumeration
order must not affect name resolution, diagnostics, layout, or generated
artifacts. A nested directory starts a separate module namespace and is not
implicitly included in its parent module.

This model keeps implementation logic easy to split across focused files
while giving external consumers and language servers one deterministic facade
to inspect. Actus controls which directory is a module root; the compiler
does not infer modules from arbitrary filesystem paths.

## 4. Visibility

Actus declarations are closed by default. The `open` keyword exports a
declaration for use by importing modules:

```act
open verb set_high() {
    ...
}

verb internal_helper() {
    ...
}
```

`open` is the only visibility keyword in the alpha design. A separate
`closed` keyword is unnecessary because closed visibility is the default.

`open` describes module visibility only. It does not imply inheritance,
dynamic dispatch, mutability, or runtime access.

## 5. Naming Conventions

| Construct | Convention | Example |
| --- | --- | --- |
| Verbs | `snake_case`, action-oriented | `read_packet` |
| Boolean verbs | `is_` or `has_` prefix | `is_ready` |
| Types and resources | `PascalCase` nouns | `MemoryPool` |
| Bindings and arguments | `snake_case` nouns | `packet_view` |
| Constants and statics | `SCREAMING_SNAKE_CASE` | `MAX_PAYLOAD` |
| ABI tags | `PascalCase` enum variants | `C`, `System`, `Wasm` |

Names must describe domain meaning. Generic names such as `data`, `thing`, and
`item` should be avoided when a precise name is available.

## 6. Semantic Roles

Actus uses four explicit roles for ownership and borrowing:

- `erg` owns a resource and is responsible for its deterministic cleanup;
- `abs` is a non-owning lexical borrow with no destructor responsibility;
- `dat` transfers ownership into another binding or function context.
- `ins` grants a temporary exclusive call-scope loan. The source owner enters
  `Suspended` access for the call and returns to mutable access afterward.

`abs` does not create runtime reference counting. Ordinary views are lexical;
an `abs` return is allowed only under the single-origin rule and remains tied
to the caller owner through a caller-scope borrow record.

Struct aggregates preserve these roles in their fields. Unmarked fields are
value fields, `erg` fields are owned mutable subresources, and `abs` fields are
read-only views. Persistent `ins` fields and `dat` fields are not permitted:
`ins` is a call-scope loan and `dat` is an operation-level transfer. Structs
are not implicitly copied; compatible whole-struct assignment transfers the
source owner and cleans the destination's previous owned value.

`perform Role for Type` is compile-time static dispatch. `abs dynamic Role` is
explicit runtime dispatch and uses a borrowed two-word fat pointer in the
order `data_ptr`, then `vtable_ptr`. Its cross-unit metadata and compatibility
rules are defined in
[ADR-0017](../decisions/ADR-0017-dynamic-role-abi-and-cross-unit-metadata.md).
Dynamic dispatch does not grant ownership or mutation capabilities.

### Explicit scalar reuse

`copy(value: abs value)` is the explicit scalar reuse operation. It requires an
`abs` argument and is accepted only for compiler-approved integer and boolean
scalar values. It does not copy buffers, strings, resources, aggregates with
cleanup obligations, or user-defined types. Ownership roles remain visible at
the call boundary; an owning call still moves its `erg` or `dat` argument.

Use this operation when the same eligible scalar must be passed to multiple
owning-role call sites. Do not use identity arithmetic such as `value + 0u32`
to express a copy. Unsupported types and calls without an explicit `abs` role
or a provably safe inferred role are rejected during semantic analysis.

If an imported generic verb named `copy` is also visible, the compiler resolves
calls by shape and provenance. A local module-scoped `copy` declaration wins;
the one-argument scalar form resolves to the scalar reuse intrinsic when no
local declaration shadows it; and the two-argument `reader`/`writer` form
resolves to the imported generic verb. This decision occurs before generic
specialization and native reanalysis.

At a static local call, an omitted call-site role may be inferred only for an
`abs` parameter receiving an `abs` binding or an `ins` parameter receiving an
`ins` binding. The compiler records the resolved role and whether it was
explicit or inferred. It never infers `erg` or `dat`, and it does not infer
through dynamic or external calls, aggregates, buffers, resources, or
ambiguous expressions. Write `abs value` or `ins value` when source-level
clarity is preferred; an explicit role remains the opt-out form. Unsafe or
ambiguous omissions continue to produce `E1016` at the argument expression.

## 7. Canonical Formatting

The canonical formatter must enforce:

- exactly four spaces for indentation;
- no tabs;
- a target line width of 100 columns;
- one space after type-annotation colons;
- no trailing whitespace;
- K&R / 1TBS braces.

```act
verb loop_example() -> Int {
    loop {
        break;
    }
    return 0;
}
```

Short calls remain on one line. Long calls place one argument per line and
use a trailing comma:

```act
erg result = configure_dma(
    channel: 2,
    buffer: rx_buf,
    interrupt_priority: 1,
);
```

## 8. Operators and bounded access

Use explicit parentheses when a mixed expression would be difficult to read.
The canonical precedence table and operand contracts are maintained in the
[operator reference](operators.md). Do not rely on implicit numeric
conversion, truthiness of integers, or unchecked array access.

```act
verb valid(abs bytes: Buffer, erg index: u32) -> u8 {
    return bytes[index];
}
```

`expr as Type` must be used when an integer value changes primitive width.
The cast is checked; it is not a formatting hint or an unchecked reinterpret.

## 9. Foreign Function Interface

Foreign declarations use an explicit ABI tag:

```act
unsafe extern "C" verb snprintf(...);
```

Raw pointer types, volatile MMIO operations, exact layout attributes, and
inline assembly are accepted future low-level features defined by
[ADR-0012](../decisions/ADR-0012-low-level-primitives-and-unsafe-boundaries.md).
They are available only inside explicit `unsafe` boundaries and do not change
the ownership semantics of safe `erg`, `abs`, and `dat` code. Foreign symbols
may retain their native naming conventions at the FFI boundary; Actus-facing
wrappers follow the Actus naming rules.

The current alpha backend supports the C ABI only. The low-level primitives
described by ADR-0012 are accepted design commitments and are not all
implemented in the current compiler.

## 10. Compatibility Rule

When the implementation and this document disagree, the discrepancy must be
resolved explicitly. A syntax or semantic change requires an update to this
document, the relevant roadmap item, tests, and user-facing documentation in
one logical change.
