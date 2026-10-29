# `meta must_use(...)`

`meta must_use("reason")` marks a verb result as requiring an explicit use at
the call site. The reason is part of the compiler diagnostic and must be a
non-empty string.

```actus
meta must_use("the caller must inspect the write status")
open verb write_frame(abs frame: Buffer) -> Result[Usize, IoError] {
    return write_frame_native(frame: abs frame);
}
```

Ignoring a marked result as a standalone expression is rejected:

```actus
write_frame(frame: abs frame);
```

The result is consumed when it is returned, assigned to a binding, passed as
an argument, or otherwise used by an expression. `must_use` does not insert
runtime code, change ownership roles, or make a result implicitly copied.

## Contract

- The form is `meta must_use("non-empty reason")`.
- It attaches to an ordinary or external verb with a return type.
- It is rejected on a verb without a return value.
- It may be used on imported external declarations so the contract remains
  visible to consumers.
- Duplicate `must_use` attributes on one declaration are rejected.
- The diagnostic is a semantic error at the ignored call site.
- The reason is documentation for the caller; it is not executable code.
- Formatter and LSP output preserve the metadata and its reason.

The attribute is intentionally narrow in its first implementation: it checks
discarded direct call expressions. Broader expression and method-dispatch
coverage must be added only with corresponding ownership and semantic fixtures.
