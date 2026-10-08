# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 19. Metadata

Metadata is written with `meta` and is compiler-facing, not a runtime value.

### Tests

```act
meta test
verb buffer_contract() -> Int {
    ...
}
```

The Actus test runner discovers test verbs according to the current source
test contract. Keep fixtures focused and deterministic.

### Target selection

```act
meta target("host")
verb host_only_entry() -> Int {
    ...
}
```

Target metadata filters declarations for a configured compiler target. Target
profiles belong to compiler/build/runtime configuration, not to the core
language syntax. Never embed a list of CPU models into a type or operator.

### Source-limit exceptions

Source-size limits measure code-bearing lines rather than raw physical lines.
Blank lines and comments are excluded, while Actus documentation strings are
counted. The preferred, split-required, and hard thresholds therefore apply to
the actual source structure and are not increased merely by adding comments.

The recommended default is to keep source files and verbs within the project
limits. If a carefully justified exception is needed:

```act
meta limitless("verb")
verb generated_protocol_decoder() -> Result[Int, DecodeError] {
    ...
}
```

At file scope:

```act
meta limitless("file")
"""Generated compatibility surface; reviewed as one boundary."""
```

The package-wide policy is configured in `Actus.toml` as part of `[package]`:

```toml
[package]
name = "example"
version = "0.1.0"
edition = "alpha"
entry = "main"
source_limits = "limitless"
```

Do not add `source_limits = "default"` redundantly. Default limits apply when
the package does not opt out. A limitless policy is a reviewed exception and
should not become the normal design strategy.
