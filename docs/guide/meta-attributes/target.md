# `meta target(...)`

`meta target("selector")` makes a declaration available only when the
configured compiler target belongs to the selected platform family.

```actus
meta target("unix")
verb unix_path_policy() -> Int {
    return 1;
}

meta target("windows")
verb windows_path_policy() -> Int {
    return 2;
}
```

The current selectors are:

- `"unix"` for Unix hosted targets;
- `"posix"` for targets with the supported POSIX contract;
- `"windows"` for Windows targets.

Target filtering occurs before semantic analysis of the selected program. A
non-matching declaration is therefore not required to type-check against the
active target, while the target selector itself must still be valid metadata.

## Rules

- The selector must be a string literal.
- An unsupported selector is a compile-time error.
- Conflicting target attributes on one declaration are rejected.
- Target metadata does not select CPU models, linker flags, or runtime
  providers; those remain part of target and package configuration.
- Do not use target metadata to hide a portable implementation accidentally.
  Add a target-specific declaration only when the target contract genuinely
  differs.

Target-specific behavior requires positive and negative fixtures for the
relevant target families. A Unix-only fixture is not evidence of Windows
behavior, and a passing host build is not evidence for an untested target.

