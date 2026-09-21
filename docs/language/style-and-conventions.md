# Actus Language Style and Conventions

Version: 0.1

Status: Accepted language design

This document defines the canonical style, visibility, import, naming, and
documentation conventions for Actus and the Arca ecosystem.

The compiler is still under active development. Rules marked as planned are
part of the accepted design but are not necessarily implemented in the alpha
compiler yet.

## 1. Comments and Documentation

The accepted comment syntax is:

```act
| configure the hardware register base
erg base_addr = 0x40011000;
```

Long explanations use an explicitly closed block:

```act
||
This describes the zero-copy transfer protocol.
||
```

Documentation comments use `#`. A sequence of documentation lines directly
before an `open` declaration documents that public API declaration:

```act
# Configure the device interrupt line.
# The declaration is visible to importing modules.
open verb configure_interrupt() {
    ...
}
```

The `#` documentation model and `arca doc` generation are planned tooling
features. They must preserve source spans and remain separate from ordinary
compiler diagnostics.

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
- module resolution is controlled by `Arca.toml` and the package graph;
- imports must not depend on arbitrary unresolved filesystem paths;
- circular imports must produce a deterministic diagnostic;
- only `open` declarations are visible across module boundaries.

The module resolver and grouped import syntax are planned compiler features.

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

Actus uses three explicit roles for ownership and borrowing:

- `erg` owns a resource and is responsible for its deterministic cleanup;
- `abs` is a non-owning lexical borrow with no destructor responsibility;
- `dat` transfers ownership into another binding or function context.

`abs` does not create runtime reference counting. It is valid only within its
lexical lifetime and cannot be returned or stored in a longer-lived owner.

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

## 8. Foreign Function Interface

Foreign declarations use an explicit ABI tag:

```act
unsafe extern "C" verb snprintf(...);
```

Raw pointer operations belong in dedicated low-level modules such as
`ffi.act`. Foreign symbols may retain their native naming conventions at the
FFI boundary; Actus-facing wrappers follow the Actus naming rules.

Additional ABI tags and pointer types are future extensions. The current
alpha backend supports the C ABI only.

## 9. Compatibility Rule

When the implementation and this document disagree, the discrepancy must be
resolved explicitly. A syntax or semantic change requires an update to this
document, the relevant roadmap item, tests, and user-facing documentation in
one logical change.
