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
an argument, used by an operator, or explicitly propagated with `?`.
Parentheses alone do not consume it. Each nested statement is checked
independently, so a call inside a conditional block cannot bypass the
contract. `must_use` does not insert runtime code, change ownership roles, or
make a result implicitly copied.

Role methods may carry the same metadata. Every `perform` implementation must
repeat the exact `must_use` contract declared by the role method. Dynamic and
generic role dispatch preserve the contract at the call site.

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
- Operator operands, `?`, field/index receivers, and dispatch receivers count
  as value-consuming contexts; grouping alone does not.
- Role implementations must preserve the declaration contract and report
  `E1123` when they add or omit `must_use`.

The compiler checks direct calls, method and role dispatch, imported and
external signatures, generic substitutions, nested expression contexts, and
role implementation parity. It does not infer `must_use` from arbitrary
user-defined conventions; the declaration is the explicit source of the
contract.
