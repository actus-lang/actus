# ADR-0052: Backend Boundary and Optional LLVM Integration

## Status

Proposed. This ADR records the boundary required before Actus adds a second
native code generation backend. The implementation baseline is the annotated
tag `checkpoint/pre-backend-refactor`.

## Context

Actus currently emits native objects through Cranelift. The compiler pipeline
is already separated into lexer, parser, AST, semantic analysis, codegen,
runtime, and target modules, but the `src/codegen` implementation is not yet a
backend boundary. Cranelift types and builders are used throughout lowering,
expression handling, control flow, aggregate memory operations, ABI emission,
runtime declarations, object emission, linker planning, and target ISA setup.

Adding LLVM directly to those modules would duplicate lowering logic and make
ownership, cleanup, layout, and ABI behavior diverge between backends. The
compiler therefore needs a small Actus-oriented contract between semantic
lowering and machine-specific code generation.

## Current implementation inventory

The current Cranelift dependency surface includes:

- `src/codegen/cranelift/isa.rs`: Cranelift ISA lookup and optimization flags;
- `src/codegen/cranelift/native.rs`: Cranelift native facade, function metadata,
  object entry points, and public emission API;
- `src/codegen/cranelift/declarations.rs`, `emission.rs`, `dependencies.rs`,
  and `ir_audit.rs`: Cranelift-specific function discovery, object emission,
  reachability, and zero-float verification;
- `src/codegen/cranelift/runtime.rs`: runtime bridge declarations and function
  identifiers;
- `src/codegen/linker.rs`: object linking and platform linker arguments;
- `src/codegen/abi.rs`, `layout/`, `types.rs`, and `structs/`: layout, ABI, and
  aggregate lowering decisions that must be split into shared contracts and
  backend implementations;
- `src/codegen/calls/`, `case/`, `expressions/`, `function_definition/`,
  `lowering/`, `enums/`, `arrays.rs`, `arenas.rs`, `cleanup.rs`, and related
  modules: lowering code that currently constructs Cranelift instructions;
- `src/codegen/model.rs`: native cleanup and unwind models that must remain
  backend-neutral even when their current consumers are Cranelift-based.

The complete source inventory is reproducible with:

```sh
rg -l "cranelift" src tests benches | sort
```

The inventory is intentionally recorded before any move so the migration can
prove that shared modules no longer import backend-specific types.

## Responsibility mapping

| Responsibility | Shared boundary | Cranelift implementation |
| --- | --- | --- |
| Backend-neutral functions, blocks, values, calls, returns, traps, cleanup, and role-aware passing | `src/codegen/contract.rs` | Consumes the contract when backend lowering is introduced |
| Deterministic symbol identity and symbol kinds | `src/codegen/symbols.rs` | Uses the identity when declaring and emitting symbols |
| Cleanup and unwind plans derived from semantic state | `src/codegen/model.rs` | Emits the plan through Cranelift cleanup lowering |
| Actus semantic lowering and source diagnostics | parser, semantic, and shared codegen interfaces | Must not import backend crates |
| Native instruction construction, target ISA setup, object emission, runtime declarations, and linker integration | Target-neutral metadata only | `src/codegen/cranelift/` |

The mapping is enforced by the architecture conformance test. Shared codegen
modules may expose Actus contracts and metadata, but they may not name
Cranelift crates. The implementation boundary is visible in the filesystem:
Cranelift imports remain below `src/codegen/cranelift/`; a future LLVM
implementation belongs below `src/codegen/llvm/`.

## Decision

Actus will introduce a backend-neutral lowering contract under `src/codegen`
and isolate concrete implementations below backend-specific modules. The
first migration keeps Cranelift as the default backend and preserves its
observable behavior. LLVM is an optional second backend, enabled through a
Cargo feature such as `llvm`.

The intended shape is:

```text
src/codegen/
├── mod.rs               # codegen facade and backend selection
├── contract.rs          # backend-neutral lowering operations and metadata
├── ir/                  # minimal Actus-oriented lowering values
├── abi.rs               # shared ABI metadata and validation contracts
├── layout/              # shared Actus layout decisions
├── target.rs            # target profiles and capabilities
├── runtime.rs           # shared runtime import contract
├── cranelift/           # Cranelift-only implementation
│   ├── mod.rs
│   ├── lowering/
│   ├── native_runtime.rs
│   ├── object.rs
│   └── linker.rs
└── llvm/                # optional LLVM-only implementation
    ├── mod.rs
    ├── lowering/
    ├── runtime.rs
    ├── object.rs
    └── linker.rs
```

The directory name is part of the architectural boundary. A file that imports
Cranelift belongs under `src/codegen/cranelift/`; a file that imports LLVM
belongs under `src/codegen/llvm/`. Shared files may not import either backend.
The shared directories contain Actus contracts and target metadata only.

Tests follow the same distinction:

```text
tests/codegen/
├── shared/               # backend-independent semantic and contract tests
├── cranelift/            # Cranelift object and execution tests
└── llvm/                 # LLVM object and execution tests
```

This layout is intentionally visible in the filesystem so a contributor can
identify the backend responsible for a file without reading its internals.

The contract represents only Actus requirements: typed values, blocks,
control flow, calls, returns, traps, cleanup actions, aggregate layouts,
runtime imports, ABI metadata, and deterministic symbols. It is not intended
to reproduce LLVM's optimizer, instruction selector, register allocator, or
object writer.

The initial contract surface is implemented in `src/codegen/contract.rs`. It
contains backend-independent function, block, value, aggregate-layout,
role-aware parameter, call, return, trap, cleanup, and symbol references. Its
validation checks layout alignment, entry-block identity, and deterministic
call-symbol presence without selecting a machine backend. Parameter passing is
represented by target-neutral direct, indirect, borrowed, exclusive-loan,
ownership-transfer, and runtime-bridge modes. Aggregate returns can require an
explicit validated return slot; register assignment and stack placement remain
backend responsibilities.
Unsupported operations use a typed diagnostic carrying the operation name,
source span, selected backend, target profile, and stable reason text. A
backend must return this diagnostic instead of approximating an unsupported
Actus contract.

### Enum and typed-failure representation

Typed failures carry an explicit `BackendEnumLayout` through the neutral
contract. The layout records the aggregate extent and alignment, discriminant
byte offset and width, payload byte offset and layout when a payload exists,
and whether the representation uses a niche pointer. The contract validates
every range, alignment, and payload-presence relationship before backend
emission. A backend therefore cannot infer a `Result` or `Option` layout from
only its type name.

The LLVM hosted emitter lowers regular aggregate enum failures with an integer
payload by materializing the declared layout, writing the discriminant and
payload at their explicit byte offsets, and returning the aggregate directly
or through the declared return slot. Niche-pointer representations are a
private backend representation. The source language does not define a `null`
literal or null-valued expression; source programs represent absence through
the typed `Option::None` variant. When a target ABI uses a niche pointer, the
selected backend may encode that typed state with a private pointer sentinel
after semantic lowering. The sentinel must not appear in Actus source syntax,
public Actus APIs, serialized state, or ownership rules. The source-level
semantic suite rejects `null` as an undeclared identifier, while backend tests
may inspect the private emitted representation.

### Direct aggregate ABI contract

The neutral `BackendLayout` describes memory size and alignment; it does not
by itself define how a direct aggregate crosses a function boundary. Direct
aggregate lowering therefore uses an explicit `DirectAggregate` passing or
return mode carrying a validated target/backend ABI descriptor before it is
enabled for a target. That descriptor must define:

- whether the aggregate is returned in registers, through a hidden return
  address, or through a target-specific mixed rule;
- how each aggregate fragment maps to integer registers or stack slots;
- the required register class, byte order, padding, and alignment for every
  fragment;
- the caller and callee lowering for direct parameters, direct returns, and
  direct aggregate calls; and
- the external C ABI declaration used by the acceptance fixture.

An implementation must not infer this contract from aggregate size alone or
silently replace a direct aggregate with an indirect pointer. The neutral
contract validates that the descriptor matches the aggregate layout and value
type. The hosted LLVM emitter accepts the explicit mode and emits the declared
aggregate type. Cranelift supports the first hosted slice when parameter and
return fragments are contiguous integer registers; it rejects stack fragments
and non-contiguous register assignments with stable diagnostics until their
target-specific placement rules are implemented. The first acceptance target
is hosted x86-64; ARM32/Thumb direct aggregate rules remain part of Gate 44.6.

### Cleanup value boundary

Semantic cleanup plans currently identify bindings and borrow records by their
semantic indexes. A native runtime bridge additionally needs the lowered value
that carries the resource address or capability handle. A backend must not
derive that value from a source name, binding index, or hidden global state.
Runtime cleanup parity therefore requires an explicit neutral mapping from each
cleanup action to a validated `BackendValueId` and, for partially moved
aggregates, an explicit field or sublayout descriptor. The mapping must retain
LIFO ordering, ownership state, and the declared layout through backend
lowering. Until that contract exists, LLVM and the contract Cranelift emitter
must reject runtime-backed cleanup actions with a stable diagnostic;
compile-time `EndBorrow` remains a no-op boundary. The full Cranelift language
lowerer has its own typed cleanup context and does not define the neutral
contract for the LLVM parity path. The neutral instruction contract now
requires a value operand for every runtime-backed cleanup action, carries the
declared resource layout, and optionally carries a bounds-checked field
sublayout. Missing operands and invalid field ranges are rejected before
backend lowering; actual runtime bridge calls remain subsequent work.

## Mandatory contracts

### Initial LLVM integration policy

LLVM is a hosted optional backend. Its implementation boundary is
`src/codegen/llvm/`, and its build activation is the Cargo feature `llvm`.
The default feature set remains `cranelift`; it must compile and operate
without an LLVM installation, LLVM headers, LLVM libraries, or LLVM Rust
bindings. Shared semantic lowering, target metadata, runtime contracts, and
linker planning must not import LLVM types.

The selected first binding is `llvm-sys 231.0.0`, matching LLVM 23.1.x, with
the binding's `strict-versioning` feature enabled. The build discovers LLVM
through `llvm-config` or `LLVM_SYS_231_PREFIX`; the default build does not
perform that discovery. The minimum supported host toolchain is LLVM 23.1.x
and the initial target scope is hosted targets. The LLVM implementation now
includes validated hosted IR lowering, native host object emission, and host
object-to-executable linking through the shared linker plan. Cross-target
target registration and freestanding startup remain separate acceptance steps.
An LLVM feature build
that lacks the required toolchain must return a typed setup diagnostic rather
than fall back to Cranelift or silently emit a different object.

### Dependency isolation

The default build must not require a system LLVM installation or compile LLVM
bindings. LLVM-specific crates, build scripts, discovery, and diagnostics must
be activated only by the explicit feature. CI must test the default and LLVM
feature configurations independently.

### ABI and target isolation

Target selection and backend selection are separate decisions. A target profile
owns pointer width, byte order, alignment, calling convention, object format,
linker flavor, entry contract, and runtime capability set. ARM32/Thumb support
must encode the selected ARM AAPCS rules, including argument registers,
callee-saved registers, aggregate passing, aggregate returns, and 8-byte stack
alignment.

The typed ARM ABI contract currently records the base AAPCS facts used by the
profile: 32-bit words and pointers, `r0-r3` argument registers, `r0-r1`
return registers, `r4-r11` callee-saved registers, and 8-byte stack alignment.
Aggregate classification and freestanding runtime/linker integration require
their own acceptance evidence before they are considered supported.

The representation contract distinguishes core-register words, explicit stack
spills with byte size/alignment/offset, and indirect return addresses. The
current canonical fixtures use one scalar word in `r0` and an aggregate stack
spill with an indirect return address in `r0`. The emitted Thumb call fixture
also verifies two generated callers with a `bl` instruction and paired
8-byte stack frames, providing stack-alignment evidence for the supported call
slice.

The LLVM object emitter initializes LLVM's complete target registry rather than
only the host-native target. The hosted test suite now emits and parses an ARM
ELF object for `thumbv7em-none-eabihf` and checks its `.text` section and Actus
entry symbol. This remains object-generation evidence; it does not establish
freestanding linking or target execution.

The ARM ABI fixture also builds scalar and two-word aggregate caller/callee
pairs using the neutral direct-call and return contract, then emits and parses
one ARM object containing both directions. The fixture confirms that scalar
and aggregate argument and return paths reach LLVM ARM object generation with
stable public symbols. It does not establish exact AAPCS register allocation,
freestanding linking, or hardware execution.

The linked-image fixture checks the final ELF's vector-table alignment and
values, Thumb reset-state bit, reset-symbol target, flash placement, absence
of relocations, absence of undefined symbols, and absence of forbidden hosted
imports. This provides object-level linker acceptance; instruction-level
runtime observation remains a separate evidence boundary.

The first ARM profile is `thumbv7em-none-eabihf`, represented by the typed
`CortexMv7em` target profile and `Thumb2` instruction mode. This profile
identifies the Cortex-M v7E-M architecture family; it does not by itself claim
that the ARM ABI, runtime provider, linker script, or hardware execution path
is complete.

The target contract now records the Cortex-M freestanding link requirements:
an explicit entry symbol, an explicit runtime provider, a linker script, and
8-byte stack alignment. These are compiler-facing requirements; the concrete
startup object, provider symbols, linker script, and link acceptance remain
implementation work.

The build configuration now provides an explicit `linker_script` path. Actus
resolves it relative to `Actus.toml` and adds `-T <path>` to GNU linker plans.
Other linker flavors reject this option until their script syntax and
acceptance contract are defined. The configuration and plan test proves only
argument construction, not successful ARM image linking.

The LLVM fixture now performs a real minimal freestanding link using an
Actus-generated `thumbv7em-none-eabihf` object, explicit Thumb startup and
`__aeabi_unwind_cpp_pr0` provider objects, and a pinned `memory.x` script.
The resulting ARM ELF is parsed for its vector table, `.text`, startup symbol,
Actus entry symbol, and absence of undefined symbols. The acceptance record is
LLVM/Clang 23.1.1, rust-lld LLD 22.1.8, and target triple
`thumbv7em-none-eabihf`.

The connected STM32H753 target was programmed with the generated ELF through
ST-LINK V3 using `probe-rs 0.32.0`, chip `STM32H753ZITx`, SWD at 50 kHz,
`--verify`, and `--reset`; the command returned exit code 0. A separate reset
also returned exit code 0. The target image therefore has programming and
reset evidence. Instruction-level runtime observation remains open because
the installed `arm-none-eabi-gdb` cannot load its Python module; no claim is
made about application-level behavior beyond successful verified programming
and reset.

### External C ABI boundary

External calls use the same validated `SymbolIdentity` and `BackendExternal`
contract as internal calls. The only foreign calling convention currently
accepted by the FFI contract is C. Imported signatures must declare every
parameter and the return type explicitly. Integer widths, pointer-width
representations, aggregate passing modes, and return modes are validated before
LLVM lowering; unsupported Actus types or ambiguous return ownership are
rejected.

The Actus role at each external parameter remains part of the ABI contract:
`abs` maps to a shared read-only borrow, `ins` and `erg` require an exclusive
mutable access path, and `dat` consumes ownership. Returned borrowed values do
not acquire ownership, while owned resource returns acquire an explicit
ownership path. The C boundary is unsafe and explicit at the source interface;
it does not infer ownership from a foreign declaration.

External symbols are deterministic Actus symbols. For example,
`SymbolIdentity::new("runtime", SymbolKind::External, "increment")` emits
`runtime__extern_increment`. Symbol visibility is explicit in the object
contract; no host library or operating-system symbol is implicitly imported.

Gate 44.7 acceptance includes a Linux fixture that compiles a standalone C
provider with Clang for `thumbv7em-none-eabihf`, emits an Actus LLVM object that
calls the declared provider symbol, and links both objects with an explicit
startup object and linker script. The fixture checks the final ARM ELF for the
provider and Actus symbols, zero relocations, and zero undefined symbols. The C
source is test input compiled by an external tool; Actus does not parse or
compile arbitrary C source.

The fixture proves external object interoperability for a scalar return path.
Aggregate C ABI interoperability, runtime cleanup calls, and serialized-state
pointer exclusion are separate concerns. The semantic pack contract rejects
`Buffer`, `String`, and `Region[...]` fields from serialized storage, and the
regression `rejects_pointer_bearing_or_dynamic_pack_fields_from_serialized_state`
records that boundary. External runtime calls remain explicit because the
neutral module must contain a declared `BackendExternal` and a matching
`BackendInstruction::Call`; the LLVM emitter does not discover or inject a
foreign symbol implicitly.

### Semantic preservation

Ownership roles, cleanup order, typed failures, enum discriminants, aggregate
layouts, Region descriptors, serialization layouts, and runtime bridge symbols
are shared Actus contracts. A backend may reject an unsupported contract but
may not silently reinterpret it.

### Evidence boundary

Host compilation proves host compilation only. ARM32/Thumb acceptance requires
target-specific object, link, and execution evidence. A successful LLVM IR
build is not evidence that a Cortex-M ABI call is safe.

## Migration sequence

1. Freeze the checkpoint and inventory the current dependency surface.
2. Define the minimal backend-neutral contract and its tests.
3. Move Cranelift implementation behind the contract without behavior changes.
4. Validate all existing Cranelift tests, objects, symbols, and benchmarks.
5. Add optional LLVM hosted lowering for the shared contract.
6. Add ARM32/Thumb target profiles and AAPCS validation.
7. Add freestanding object/link/target evidence.

Each step is a separate reviewable change. LLVM integration must not begin by
rewriting the semantic analyzer or by adding a second implementation of Actus
ownership rules.

## Consequences

- Cranelift remains usable without LLVM installed.
- The first refactor may move many files without changing generated behavior.
- Backend-neutral contracts add an explicit maintenance boundary.
- LLVM dependency and build costs are isolated behind an opt-in feature.
- ARM32/Thumb support becomes possible, but it requires a separate ABI and
  target validation effort.

## Acceptance evidence

- [x] `checkpoint/pre-backend-refactor` identifies the pre-refactor main
      commit.
- [x] The current Cranelift dependency surface is recorded above.
- [x] The proposed backend-neutral boundary and migration order are recorded.
- [x] The shared contract is implemented and covered by backend-independent
      tests for layouts, role-aware passing, return slots, diagnostics, and
      control-flow targets.
- [ ] Cranelift is isolated without observable behavior changes.
- [ ] Optional LLVM hosted lowering passes the shared contract suite.
- [ ] ARM32/Thumb output passes target-specific ABI and execution evidence.
