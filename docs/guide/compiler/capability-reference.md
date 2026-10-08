# Complete compiler capability reference

This page preserves the detailed capability and status reference from the original guide.

### Current systems-language capability status

The current systems-language capability set is compiler-owned and
target-neutral; CPU identities belong in target and runtime configuration,
not in Actus syntax.

#### Const generics

The initial `Usize` const-generic slice works across parsing, semantic
resolution, bounded array capacity, aggregate layout, generic identity, native
specialization, and strict/native acceptance. Use literal arguments such as
`Fabric[8]`. Do not use runtime values or assume arbitrary const expressions
are implemented.

Const parameters may also be used as type arguments in generic verb
signatures and bodies, and their specialized values may be read in the verb
body:

```act
struct Storage[N: Usize] {
    erg values: Array[u32, N],
}

verb clear[N: Usize](ins storage: Storage[N]) {
    storage.values[0] = 42u32;
}

verb capacity[N: Usize](ins storage: Storage[N]) -> u32 {
    if 0u32 < (N as u32) {
        return N as u32;
    }
    return 0u32;
}

verb main() -> Int {
    erg storage: Storage[4] = Storage[4] {
        values: Array[u32, 4](),
    };
    clear(storage: ins storage);
    return capacity(storage: ins storage) as Int;
}
```

The const argument must be a positive compile-time `Usize` literal or a
declared const parameter. Runtime expressions, zero capacities, reference
roles, and nested arguments remain invalid. `Buffer` is a compiler-owned
builtin type name; use a domain-specific name such as `Storage` for a user
defined generic aggregate.

Generic verbs may call other generic verbs without manually materializing the
helper. The compiler propagates the concrete caller substitution transitively:

```act
verb inner[N: Usize](ins storage: Storage[N]) -> u32 {
    return N as u32;
}

verb outer[N: Usize](ins storage: Storage[N]) -> u32 {
    return inner(storage: ins storage);
}
```

When `outer` is reached as `outer[4]`, the compiler also materializes the
reachable `inner[4]` instance before native declaration and lowering. This
propagation is deterministic and deduplicated; do not add duplicate helper
verbs or handwritten concrete wrappers.

When a specialized generic verb has an aggregate parameter such as
`Storage[256]`, the complete concrete aggregate identity is retained through
native layout and ABI planning. This includes aggregates containing bounded
arrays of packed elements. The same contract applies to `ins` and `abs`
parameters, direct concrete calls, facade imports, and nested generic calls.
Typed aggregate locals inside the specialized verb receive the caller's const
substitution before native lowering; source-level concrete wrapper functions
are not required.

Each concrete generic call site remains part of the specialization record even
when several call sites use the same canonical instance. The compiler may
reuse one native specialization for those calls, but it must retain every
source span needed to rewrite each call before native dependency collection.
Consequently, two calls such as `read_capacity[4]()` in one native call graph
must both lower to the specialized symbol. A generic-instance cache must not
deduplicate distinct call sites solely by canonical type arguments and caller.

Const generic parameters are also valid read-only compile-time values inside
case guards. They may be used directly or through a cast and are resolved
before native lowering:

```act
verb choose[N: Usize]() -> u32 {
    return case true {
        true if 0u32 < (N as u32) => N as u32,
        _ => 0u32,
    };
}
```

The compiler specializes `choose[4]` with the concrete value `4`; `N` is not
a mutable runtime binding and cannot be assigned, borrowed, or transferred.
Case-guard access must remain deterministic across direct and transitively
specialized generic verbs.

#### Runtime-backed logical regions

Use `Array[T, N]` when all bounded storage is part of the value. Use
`Region[T]` when the logical length is larger than the resident window and
access must remain explicit and checked. A region is a distinct owned resource;
it is not an implicit array conversion, an unbounded heap, or a filesystem
handle.

```act
import std::region;

verb read_first(dat backing: Buffer) -> Result[Int, RegionError] {
    erg opened = region_open[u32](
        backing: dat backing,
        logical_length: 1048576u64,
        window_start: 0u64,
        window_count: 16u64
    );
    return case dat opened {
        Err(error) => Err(error),
        Ok(region) => {
            erg destination: Buffer = Buffer[4];
            erg result = region_read[u32](
                region: abs region,
                index: 0u64,
                destination: ins destination
            );
            region_close[u32](region: ins region);
            result
        },
    };
}
```

`region_open` consumes the backing buffer with `dat`. The element type must be
fully sized so `size_of[T]()` produces a deterministic stride. `region_read`
uses an `abs` region and an `ins` destination; `region_write` uses an `ins`
region and an `abs` source. Logical bounds, resident-window bounds, byte
offsets, exact buffer sizes, capability handles, and generations are checked
before access and return `RegionError` on failure.

`region_publish` advances the published generation. `region_cancel` restores
the last published resident bytes. `region_close` releases the capability
exactly once, while lexical cleanup releases an owned region that leaves scope.
These operations do not perform implicit filesystem I/O or allocate an
unbounded collection.

#### Array return ABI

Returning `Array[T, N]` uses the same caller-owned return-slot ABI as other
indirect aggregate values. The caller allocates the destination slot, the
callee copies the complete bounded array into it, and the caller may then
index the returned value normally:

```act
verb make_values() -> Array[u32, 2] {
    erg values: Array[u32, 2] = Array[u32, 2]();
    values[0] = 41u32;
    values[1] = 42u32;
    return values;
}

verb main() -> Int {
    erg values: Array[u32, 2] = make_values();
    return values[0] as Int + values[1] as Int;
}
```

This prevents a returned array from referring to callee-local stack storage.
The ABI contract covers signature generation, call lowering, caller slot
allocation, callee copy-out, cleanup, and indexed access. Validate changes
with both small arrays and systems-sized arrays such as `Array[u32, 8]`.

#### Boolean literal expressions

`true` and `false` are first-class `Bool` expressions in local initializers,
returns, call arguments, aggregate fields, array elements, conditions, case
expressions, nested blocks, and value-producing branches:

```act
verb accepts_packet(abs packet: Buffer) -> Bool {
    erg linked: Bool = false;
    if linked || length(packet) > 0 {
        return true;
    }
    return false;
}
```

Boolean literals retain their source spans for diagnostics, formatter output,
hover, semantic tokens, and the LSP semantic model. They do not coerce to
integers. `&&` and `||` remain short-circuiting and require `Bool` operands.

#### Nested places, calls, and control flow

The parser and AST represent complete writable place chains without losing
selector order:

```act
struct Column {
    erg axon_0: u32,
}

struct Fabric {
    erg columns: Array[Column, 2],
}

verb notify(erg value: u32) -> Void {
    return;
}

verb update(erg fabric: Fabric, erg index: Usize, erg slot: u32) -> Void {
    if index == 0 {
        fabric.columns[index].axon_0 = slot;
        notify(value: fabric.columns[index].axon_0);
    }
    return;
}
```

Supported and tested forms include identifier assignment, field assignment,
indexed assignment, mixed index/field chains, nested indexed aggregate places,
calls inside nested statement blocks, all ownership argument roles, `?`
propagation from nested calls, nested `if`/`else` expressions, typed branch
joins, and diverging branches such as `return`, `break`, and `continue`.
The left-hand place of a compound assignment is evaluated exactly once; native
lowering follows address calculation, load, operation, and store through that
same place.

Case subjects may also be readable aggregate places, including a struct field
or an indexed element:

```act
enum State { Ready, Busy, }
struct Slot { state: State, }

verb inspect(erg slot: Slot) -> Int {
    return case slot.state {
        State.Ready => 1,
        State.Busy => 2,
    };
}
```

The semantic analyzer resolves the root binding of the place and applies the
case borrow or ownership rule to that binding. `case abs` remains read-only,
and `case dat` still requires an owned movable subject. A field or indexed
place does not bypass borrow tracking or turn a temporary expression into an
owner.

#### Package-aware test execution

`actus test` analyzes test sources through the package module graph. When a
canonical facade exists, the test file is compiled as part of that aggregate,
so sibling declarations are visible in the same way they are for `actus
check` and `actus build`.

When adding or moving tests:

- keep the test in a source tree discovered by the package manifest;
- expose sibling declarations through the canonical directory facade;
- do not make a test pass by parsing the test file as an isolated module;
- collect test metadata from the original source while compiling the complete
  module aggregate;
- verify both test discovery and package-visible type resolution.

This is required for tests that refer to declarations such as
`Fabric` or `Array[Record, 64]` from sibling modules.

#### Typed native control-flow joins

An `if` or `case` branch that produces a value must preserve its declared
native type through every nested block. A fixed-width value such as `u32` must
not become `Int` merely because it passed through a branch, call result, index
expression, or native merge block.

Apply these rules when writing or reviewing control flow:

- value-producing branches must have compatible native types;
- `return`, `break`, `continue`, and typed `?` paths are diverging paths and
  do not participate as ordinary values in a join;
- branch-local assignments to an existing owner must be represented by a
  native merge value, not discarded branch-local state;
- the same owner must be cleaned exactly once after a merge or direct owned
  return;
- a semicolon-terminated final expression is value-producing only where the
  enclosing expression grammar explicitly permits it.

For indexed native code, preserve this sequence:

```text
evaluate base and index once -> bounds check -> calculate address -> load/store
```

Never recalculate a side-effecting base or index for a later field access. The
rule applies to ordinary arrays, pack arrays, nested structs, and array-backed
pack storage.

#### Native IR verification

When a workload promises integer-only or zero-floating-point execution, inspect
the generated Cranelift IR rather than relying on source appearance or object
byte-pattern guesses. A valid audit checks:

- instruction opcodes for floating-point operations and constants;
- operand types, including floating-point comparison operands whose result is
  an integer condition;
- result types, including floating-point vector lanes where applicable;
- one integer-only fixture and one float-positive fixture.

The compiler owns this audit at the native lowering boundary. Backend-focused
tests can enable it with the native configuration builder:

```rust
let configuration = NativeBackendConfiguration::default()
    .with_no_float_ir_verification();
```

The default remains disabled so existing floating-point native workloads keep
their documented behavior. With the option enabled, every generated native
function is inspected after Actus lowering and before object emission. A
failure identifies the owning function and the detected floating-point IR
instruction identities; it is not inferred from disassembly or byte-pattern
heuristics. The compiler-level evidence is covered by
`test_zero_float_verification_inspects_generated_integer_ir`,
`zero_float_verification_rejects_generated_float_ir`, and the unit audit tests
in `src/codegen/native/ir_audit.rs`.

For a package-wide contract, configure the same compiler-owned audit in the
manifest rather than enabling it only in an individual backend test:

```toml
[build]
verify_no_float_ir = true
```

This package policy is forwarded to every Actus-generated native object unit,
including module, generic, and performance specializations. The acceptance
workflow should run `check --strict` and `test --strict` first, then build both
an object and an executable. `check` validates source and semantics but does
not itself generate IR; the zero-float claim is established by the native
build report before object emission. The package-level evidence is covered by
the positive and negative zero-float CLI workflow tests in
`tests/cli_workflow.rs`.

The contract covers generated Actus IR only. Separately linked runtime
objects, external ABI implementations, compiler intrinsics, and platform
libraries require their own audit and must not be presented as covered by
`verify_no_float_ir`.

#### Array-backed packed storage

The compiler supports bounded byte-array storage for hardware and binary-layout
packs. The production-supported form is `Array[u8, N]`; it is a distinct pack
layout, not an ordinary array workaround. The capacity is compile-time bounded,
the storage is inline, and the pack's total width is `N * 8` bits.

The following 64-byte/512-bit shape is accepted and natively lowered:

```act
pack Record {
    erg storage: Array[u8, 64];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg limit: u8 at 8;
        erg low_word: u64 at 64;
        erg high_word: u64 at 128;
        erg link_0: u32 at 192;
        erg link_7: u32 at 416;
        erg back_link: u32 at 480;
    }
}
```

The complete readiness fixture contains a full 512-bit field map with explicit
fixed-width values, words, and links. Field offsets remain explicit and must
cover the storage contract without overlap or uncovered bits.

Array-backed packs provide:

- checked indexed byte reads, writes, and compound assignments;
- bit-packed field reads/writes with masking, shifting, width validation, and
  little-endian mapping;
- fixed-width `u8`, `u16`, `u32`, and `u64` field arithmetic;
- copy, move, return, aggregate initialization, and bounded inline layout;
- `erg`, `abs`, and `ins` ownership behavior through indexed storage places;
- deterministic bounds traps for invalid runtime indexes and compile-time
  diagnostics for statically impossible indexes;
- Buffer snapshot/restore through ordinary Actus operations without a hidden
  heap allocation;
- native executable and object emission without a C serialization bridge.

#### Indexed fields inside packed values

A packed value may contain one bounded, fixed-width array field when the field
is fully covered by the declared storage:

```act
pack Example {
    erg storage: Array[u8, 32];
    layout little;
    fields {
        erg links: Array[u32, 8] at 0;
    }
}
```

The array field occupies `element_width * count` bits beginning at its
declared base offset. It is not a dynamic collection, slice, pointer, or
separate allocation. The compiler records the element type, fixed count,
total width, offset, and ownership role in semantic and LSP metadata.

Use the ordinary checked indexing syntax for reads, writes, and compound
updates:

```act
abs value: u32 = example.links[index];
example.links[index] = next_value;
example.links[index] += 1u32;
```

Constant indexes outside `[0, count)` are rejected during semantic analysis;
runtime indexes use the normal checked bounds path. Immutable pack views
cannot be mutated. Indexed fields compose with arrays of pack values, so
`cells[slot].links[index]` remains an ordinary bounded place expression.

Native indexed pack fields require byte-addressable storage, a byte-aligned
base offset, and an element width that is a whole number of bytes. Little and
big endian layouts use the declared byte order for element loads and stores.
Unsupported representations fail with a compiler diagnostic rather than
falling back to unchecked pointer arithmetic. Existing scalar bit-packed
fields keep their masking and shifting lowering.

For dynamic indexed byte loops, use a supported integer index such as `u32`:

```act
verb snapshot(ins output: Buffer, abs record: Record) -> Int {
    erg index: u32 = 0;
    loop {
        if index >= 64 {
            break;
        }
        append(output, record.storage[index]);
        index += 1;
    }
    return index as Int;
}
```

`Usize` remains the tested const-generic domain for declarations such as
`Array[T, N]`. Do not silently substitute runtime capacity, `Array[u16, N]`,
or an unbounded storage representation for the supported packed-storage
contract.

#### Explicit scalar reuse

Use `copy(value: abs value)` when an eligible scalar must be reused at an
owning call boundary. The compiler accepts `Int`, `Bool`, and fixed-width
integer values. The `abs` role is explicit for this intrinsic and the operation
does not consume the caller. Do not use identity arithmetic such as `value + 0u32` to express
reuse. Buffers, strings, resources, cleanup-bearing aggregates, and
unsupported user-defined values are rejected with `E1021`; a missing explicit
`abs` role or a missing safe inferred role is rejected with `E1016`.

Static local calls support a narrow ownership inference profile. When a call
argument omits its role, the compiler may infer `abs` only from an `abs`
binding and `ins` only from an `ins` binding that is active and mutable. The
inference is resolved before ownership validation and native lowering. It does
not apply to `erg` or `dat`, dynamic or external calls, aggregates, buffers,
resources, or ambiguous expressions. Explicit `abs value` and `ins value`
remain valid and are the source-level opt-out when the ownership intent should
be visible. An unsafe omission uses the stable `E1016` argument-role diagnostic.

For a direct static call argument, the compiler may materialize a hidden
`erg` local when the source expression is a pure scalar expression and the
argument explicitly uses `abs`. This keeps small arithmetic and bounded index
expressions readable without changing ownership or evaluation order:

```act
verb read(abs value: u32) -> Int {
    return value as Int;
}

verb main() -> Int {
    erg values: Array[u32, 2] = Array[u32, 2]();
    values[1] = 41u32;
    erg index: u32 = 1u32;
    return read(value: abs values[index]);
}
```

The compiler-owned local is equivalent to an explicit scalar binding for the
duration of that call. It is generated in deterministic source order, receives
`erg`, and is passed to the callee as `abs`. Generated names are reserved
internally and avoid collisions with source bindings. Formatter and LSP
semantic declarations do not display the hidden local. The initializer may
contain scalar literals, arithmetic, bitwise operations, comparisons, casts,
grouping, and bounded scalar indexing. Calls, method calls, buffers,
aggregates, resources, mutation, borrows, conditional expressions, and
ownership transfers remain explicit source forms.

Validated scalar constants are handled by the same normalization boundary when
they are passed with an explicit `abs` role. The compiler materializes the
constant as a hidden scalar `erg` local before native constant inlining, then
passes that local as `abs`. This preserves the source ownership contract and
prevents native preparation from turning `abs CONSTANT` into an invalid
borrow of a literal (`E1016`). Constants are not generalized into runtime
storage, and non-scalar or resource constants remain outside this rule.

The generated form has the same native text as its equivalent explicit local,
and its object relocations contain no allocation reference. A prior move or
borrow rule is still diagnosed by ordinary semantic analysis; normalization
does not repair invalid ownership.

When an imported generic verb named `copy` is visible, dispatch is determined
by call shape and declaration provenance. A local module-scoped `copy`
declaration has precedence. The one-argument scalar form
`copy(value: abs scalar)` uses the compiler-checked scalar reuse intrinsic when
no local declaration shadows it. The two-argument `reader`/`writer` form uses
the imported generic standard-library verb. Generic specialization must occur
only after this dispatch decision so unresolved parameters cannot reach native
semantic reanalysis. This rule does not relax the explicit `abs` requirement
or the supported scalar type set.

Indexed selection remains bounded through `values[index]`. Constant indexes
outside `0 <= index < N` produce `E1085`; computed indexes retain the native
bounds check. Indexed views preserve the source ownership role, so an
exclusive `ins` view cannot be created from an `abs` owner (`E1016`).

The formatter emits array-backed packs in canonical multiline form while
preserving storage type, field offsets, layout, and `"""` documentation. The
LSP exposes storage width, byte capacity, layout, field metadata, hover,
completion, semantic tokens, definition/navigation, rename, and overlay
reindexing for these packs. Pack diagnostics retain stable codes and exact
source spans, including invalid storage (`E1070`), out-of-range fields
(`E1072`), overlap (`E1073`), constant index bounds (`E1085`), and ownership
violations (`E1051`).

The canonical end-to-end evidence is maintained in:

```text
the repository's strict package and executable acceptance tests
```

That acceptance runs strict check/test/format validation, host-native build and
execution on `x86_64-unknown-linux-gnu` with exit status `126`, object emission
with `.text` and `main`, and rejected fixtures for overlap, field bounds,
invalid storage elements, runtime capacity, indexed bounds, and ownership.
Do not claim a packed-storage feature is complete from parsing alone; require
semantic, native, formatter, LSP, rejected-fixture, and executable evidence.

#### Pack values as array elements

Declared `pack` types are also valid typed elements of bounded arrays. The
supported form is `Array[Pack, N]`, where every element is stored inline using
the pack's complete native layout and stride. A pack is not lowered as a
pointer, opaque handle, or manually flattened byte array.

```act
pack Cell {
    erg storage: Array[u8, 2];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg tail: u8 at 8;
    }
}

verb main() -> Int {
    erg cells: Array[Cell, 2] = Array[Cell, 2]();
    cells[1] = Cell { storage: Array[u8, 2](), };
    cells[1].marker = 41u8;
    cells[1].marker += 1u8;
    return cells[1].marker as Int;
}
```

The compiler registers pack declarations before validating struct and array
references. Array layout resolution preserves pack identity, field metadata,
alignment, size, and the complete inline element stride. Dependent storage
arrays are laid out before arrays containing those packs, so an
`Array[Record, 64]` layout can use a 64-byte array-backed `Record`
without a runtime descriptor.

Pack-array places support:

- checked indexed element access and field access such as `cells[index].marker`;
- field assignment and compound assignment through the same checked address;
- one evaluation of the array base and index before native address lowering;
- `erg`, `abs`, and `ins` ownership behavior, including an `ins` loan of one
  pack element that restores the original array owner;
- aggregate initialization, copy/move/return paths, and cleanup boundaries;
- deterministic rejection of unknown pack types, invalid layouts, impossible
  indexes, illegal aliases, use-after-move, and layout/stride overflow.

The corresponding evidence is:

```text
tests/semantic/packs.rs::accepts_pack_types_as_bounded_array_elements
tests/semantic/packs.rs::rejects_unknown_pack_types_as_array_elements
tests/arrays_cli.rs::executes_array_of_array_backed_packs_natively
tests/arrays_cli.rs::executes_compound_assignment_through_array_of_packs_natively
tests/arrays_cli.rs::preserves_ins_loan_through_an_array_of_packs_natively
tests/formatter/suite.rs::formats_pack_array_element_types_idempotently
tests/lsp/semantic_intelligence.rs::lsp_accepts_pack_types_as_array_element_types
```

This capability is complete only when these semantic, native, formatter, and
LSP checks remain green together with strict checking, object emission, source
limits, documentation checks, and the full repository test suite.

#### Formatter and LSP parity

Implemented syntax is complete only when the compiler and tooling agree. The
formatter preserves Actus `"""` documentation strings, imports, declaration
order, selector structure, and reparsability. The LSP understands const
generic parameters, Boolean literals, nested fields/places, diagnostics,
hover, definitions, semantic tokens, formatting, versioned overlays, and
malformed nested documents without process termination.

#### Verification boundary

The verification workflow covers the combined systems-language contract:

- strict package checking succeeds;
- accepted and rejected ownership cases are tested;
- formatting check succeeds without moving or deleting documentation;
- native execution succeeds;
- LSP analysis remains synchronized with the workspace source;
- source limits, public documentation, and native IR verification checks pass.

This status describes the implemented compiler capabilities and their required
verification evidence. Deferred language features require separate designs,
implementations, and tests before they may be used.
