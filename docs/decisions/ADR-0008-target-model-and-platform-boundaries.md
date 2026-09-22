# ADR-0008: Target Model and Platform Boundaries

- Status: Accepted
- Date: 2026-09-22
- Scope: Actus compilation targets and platform runtime contracts

## Context

Actus must support hosted systems and freestanding platforms without coupling
the compiler frontend to a particular operating system or external hardware
project. A target describes compilation requirements and platform runtime
contracts; it does not contain external library implementations.

## Decision

Actus separates platform concerns into the following layers:

```text
target profile
├── architecture
├── environment
├── ABI
├── pointer width
├── linker
├── startup contract
└── runtime capabilities
```

## Target Profile

A target profile describes the properties required to compile and link a
program:

- instruction-set architecture;
- hosted or freestanding environment;
- calling convention and ABI;
- pointer width and fundamental layout rules;
- linker command and linker inputs;
- startup contract;
- runtime and standard-library capabilities.

For example, `armv7m` identifies a Cortex-M-class architecture. It does not
identify any concrete hardware configuration.

An indicative target specification is:

```toml
[target]
name = "armv7m-freestanding"
architecture = "armv7m"
environment = "freestanding"
abi = "c"
pointer_width = 32
linker = "arm-none-eabi-ld"
startup = "runtime/startup.act"
profile = "freestanding"
```

## Declarative Target Specifications

Target descriptions must be fully declarative and stored in target
specification files, such as `.toml` files. Adding a target must normally
mean adding a validated specification file, not adding a new hardcoded
`match` or `if`/`else` chain to compiler code.

The compiler must:

1. load the selected target specification;
2. validate its schema and required fields;
3. derive the compilation, ABI, linker, and runtime configuration;
4. reject unsupported or inconsistent combinations before code generation.

Target-independent compiler phases must consume typed target data rather than
matching directly on target-name strings. Any target-specific procedural
behavior must live behind an explicit runtime, linker, or backend contract.

## Environment and ABI

The environment identifies the execution model:

- `hosted` provides an operating-system or hosted runtime contract;
- `freestanding` does not assume an operating system or libc.

The ABI defines the binary contract, including calling convention, register
usage, pointer representation, integer layout, alignment, and return-value
rules. Environment and ABI are independent fields in a target specification.

## Linker and Startup

The target specification selects the linker interface and startup contract.
The linker is an external or separately defined build component; its behavior
must not be embedded in the lexer, parser, semantic analyzer, or general AST.

Startup code belongs to the selected target runtime. It may establish stacks,
initialize data sections, install interrupt vectors, and transfer control to
the Actus entry point. Hosted and freestanding startup contracts are distinct.

## Consequences

Declarative target specifications make target support extensible and auditable
without growing hardcoded compiler conditionals. The explicit target boundary
keeps language semantics and frontend code independent of platform-specific
hardware projects and libraries.
