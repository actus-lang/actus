# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 17. Modules, imports, and facades

### 17.1 Importing

Use extensionless canonical module paths:

```act
import std::io;
import std::fs;
import std::path;
```

The package/build configuration resolves the module package. Do not hard-code
an absolute host path to `library/std` in application source.

### 17.2 Canonical facades

A directory module has one facade whose filename matches the directory:

```text
library/std/src/io/io.act
library/std/src/fs/fs.act
library/std/src/path/path.act
```

The facade re-exports siblings with extensionless `open` entries:

```act
open stdout;
open stdin;
open error;
```

Sibling files share internal module scope. External users see only declarations
exposed through the canonical facade. Direct sibling bypasses, missing
facades, duplicate declarations, and malformed exports must be rejected.

### 17.3 Package configuration facade

`src/config/config.act` is the reserved package configuration facade when a
package uses source-level configuration:

```act
// src/config/config.act
open values;

// src/config/values.act
open const DEFAULT_THRESHOLD: u8 = 30u8;
```

Normal package code consumes it through `import config;`. Configuration files
are compile-time-only and may contain typed constants plus supporting
type-level declarations, but they must not import application, runtime,
hardware, or standard-library modules, and must not declare verbs, external
verbs, roles, or performances. The canonical facade controls visibility;
private constants and direct child-module imports remain unavailable.

Use `Actus.toml` for package, build, target, and runtime configuration. Do not
put mutable runtime state or deployment settings in `src/config`. The compiler,
formatter, LSP, test runner, and native backend must all resolve this facade
through the same module boundary.

#### 17.3.1 Configuration constants in native modules

Configuration constants remain compile-time values after they cross the
facade boundary. Native lowering must resolve them in the same public export
namespace used by semantic analysis; replacing them with source-level
literal workarounds is incorrect.

The native object plan must collect compile-time constants from the full
transitive canonical-facade dependency graph. Only public exports may enter
the imported object program; private constants and facade-bypass declarations
remain unavailable. Constants must be materialized before native identifier
lowering and must never become runtime storage or native ABI symbols.

This contract also applies through nested module facades. A child module may
import the package configuration facade and use its public constants when the
final native object is emitted for the parent module. The compiler must carry
the constant into the corresponding module object before native semantic
analysis and lowering.

Package constants may be used in all supported compile-time expression
contexts, including:

- scalar initializers and assignments;
- struct and pack literals;
- predicates, `case` guards, and conditional expressions;
- array capacities and generic const arguments where the type contract allows
  them;
- indexed expressions and layout-related values.

A constant identifier is not a runtime binding and has no ownership state.
Using a constant as an `erg` field or aggregate initializer value must not
attempt to move, borrow, or drop a runtime owner. Constants remain typed and
range-checked during semantic analysis, then are inlined or otherwise
materialized before native lowering.

When changing package configuration or module aggregation, verify the full
boundary rather than only source checking:

```sh
actus check --strict
actus build --strict --emit obj
actus build --strict --emit exe
actus test --strict
```

At least one regression test must cover a constant imported through a nested
facade and used in an aggregate initializer or predicate. The test should
verify native execution, not only semantic acceptance.

#### 17.3.2 Native dependency closure

Native emission starts from the requested entry verb or public facade roots
and computes a deterministic transitive closure of Actus verbs. Reachable
private helpers are emitted in their owning object; unreachable private verbs
are omitted. Generic verbs are specialized before declaration and lowering,
and concrete instances retain deterministic names across root-to-module
linker bindings.

The closure includes nested blocks, conditional expressions, indexed places,
method/performance calls, aggregate return dependencies, and external bridge
declarations. Built-in constructors and runtime intrinsics are not Actus verb
bodies. An unresolved native dependency fails closed at the originating call
span and identifies both the caller and missing helper; it must not be hidden
until linking.

Object builds, executable builds, and the test runner use this contract.
For imported module objects, native roots come from the resolved facade export
set, including concrete generic instances when available; they must not be
inferred from source-limit metadata or from a declaration's local `open` flag.
Formatter and source-limit checks remain independent of native reachability.
Changes to call collection, generic specialization, module facades, or symbol
bindings require semantic, native, and multi-object regression evidence.

#### 17.3.3 Shared resolution boundaries

`actus check --strict` and the LSP use the shared module/facade semantic
resolution contract. They must resolve sibling declarations, nested facades,
visibility, and public exports consistently, but they intentionally stop before
native code generation.

`actus test --strict`, object builds, and executable builds use the shared
native dependency-closure contract described above. They must start from the
same selected public roots, materialize the same reachable private and generic
dependencies, and preserve the same symbol and diagnostic identity across
surfaces. Do not implement a separate dependency scanner for one command or
force semantic-only commands to invoke code generation.

### 17.4 Visibility

`open` on a declaration or sibling export is the Actus visibility mechanism.
Do not use Rust `pub`, Go `export`, or C header conventions in Actus source.
Fields do not have a separate `pub` keyword; aggregate visibility follows the
declared facade and type contract.

### 17.5 Hierarchical facades

When a module grows beyond a single responsibility, use nested canonical
facades instead of making implementation files independently importable:

```text
src/control/control.act
src/control/runtime/runtime.act
src/control/runtime/safety.act
```

`control.act` may contain `open runtime;`, and `runtime.act` may contain
`open safety;`. External code imports only `control`. Every child directory
must contain a matching `<directory>/<directory>.act` facade. Child siblings
share internal scope, while only declarations explicitly opened through the
facade chain are public.

Valid application code uses `import control;`. Direct child paths such as
`import control::runtime;`, implementation paths such as
`import control::runtime::safety;`, and facade filenames as import segments
are invalid because they bypass the parent-controlled API. Keep the hierarchy
target-neutral and responsibility-oriented; filesystem enumeration must never
silently widen the public namespace.
