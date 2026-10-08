# Source files and project layout

An Actus package keeps project configuration, source code, tests, examples, and
build artifacts in separate locations.

A common package layout is:

```text
project/
├── Actus.toml
├── Actus.lock
├── src/
│   ├── main.act
│   └── module/
│       ├── module.act
│       └── implementation.act
├── tests/
├── examples/
└── docs/
```

`Actus.toml` selects the package source root, entry point, runtime profile,
target, and build settings. `Actus.lock` records resolved package state.

## Actus source files

A source file may contain declarations such as:

- constants;
- structs;
- packs;
- enums;
- verbs;
- external declarations;
- module directives;
- role and performance declarations.

Keep one responsibility per source file. A directory module has a canonical
facade whose filename matches the directory. The facade defines the public
boundary; sibling files contain related implementation declarations.

## Naming

Use names that describe domain meaning:

```actus
erg destination_index: u32 = 0u32;
erg descriptor_checksum: u16 = 0u16;
```

Use `PascalCase` for types and enum variants, `snake_case` for verbs, fields,
locals, and modules, and `SCREAMING_SNAKE_CASE` for constants.

## Source limits

The repository applies source-size limits to keep responsibilities readable.
Split a file by domain responsibility before it reaches the hard limit. A
triple-quoted documentation block is Actus source and belongs to the
 declaration it describes.

The exception mechanism for generated or specifically approved sources is
described in the compiler and contribution documentation. It does not replace
ordinary decomposition.

## File references

Use repository-relative paths in documentation and examples. When a page
refers to implementation details, link to the relevant source directory,
standard-library facade, test, ADR, or roadmap entry.
