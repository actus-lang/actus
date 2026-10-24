# Imports and module namespaces

Imports resolve through canonical facades. An import never grants access to a
private sibling or bypasses a parent facade. The preferred form binds a module
namespace explicitly, so the owner of each symbol remains visible at the call
site:

```actus
import config;

config::MAX_VALUE;
```

## Import forms

The compiler accepts one import item or a unified import block. A plain import
binds the final module segment as a local namespace:

```actus
import driver::i2c;

i2c::write(...);
```

Grouped imports are equivalent to separate namespace bindings and are useful
when several child modules come from one parent:

```actus
import driver::{i2c, uart};

i2c::write(...);
uart::send(...);
```

An `as` clause aliases a module or namespace binding. It cannot rename an
individual verb, type, constant, enum variant, or field:

```actus
import telemetry::uart as telem_uart;

telem_uart::send(...);
```

The complete form places all external imports in one block. `open` is allowed
there when an external consumer intentionally needs selected public
declarations in unqualified scope:

```actus
import {
    driver::{i2c, uart},
    telemetry::uart as telem_uart,
    open sensors::imu,
    open config::{MAX_SPEED, MIN_VOLTAGE},
};
```

`driver::{i2c, uart}` creates the local namespace bindings `i2c` and `uart`.
The alias applies only to the imported namespace. Grouped items are checked
independently, including their local names and source spans.

## `import` and `open` scopes

In external consumer code, standalone `open module;` is not valid. Write
`import { open module };` instead. A plain `import module;` creates a namespace
binding; it does not inject the module's declarations into the surrounding
scope.

Directory facades are a separate scope. Inside a canonical facade, `open`
remains the declaration used to re-export public declarations from sibling
files:

```actus
open reader;
open writer;
```

This facade syntax is not an external consumer import and remains valid for
every public re-export. External qualified lookup always uses the imported
namespace and never depends on ambient facade state.

## Binding and resolution rules

- Every path segment is resolved through the corresponding canonical facade.
- A child without its required canonical facade cannot be imported directly.
- Private declarations and filesystem paths are never importable.
- Repeating a local import binding is an error, including repeating the exact
  same path. Different paths may not share a final namespace or alias.
- An alias is a namespace alias only; it does not rename declarations.
- Qualified lookup is deterministic and independent of declaration order in the
  consuming file.

When migrating an external consumer, change:

```actus
open config;
MAX_SPEED;
```

to:

```actus
import { open config };
MAX_SPEED;
```

Or, when unqualified access is not required:

```actus
import config;
config::MAX_SPEED;
```

If an import fails, check the module directory, canonical facade filename,
facade export, declaration visibility, and package source root in that order.
Do not duplicate the implementation or add an unrelated wrapper to hide a
facade error.
