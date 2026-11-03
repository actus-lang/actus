# `meta deterministic`

`meta deterministic` declares that a verb's observable result must not depend
on external nondeterministic sources. The compiler rejects calls carrying the
`filesystem`, `network`, or `entropy` effects, and it rejects all
floating-point use in the deterministic contract.

```actus
meta deterministic
verb stable_hash(abs input: Buffer) -> Int {
    return hash(input: abs input);
}
```

## Contract

- The form is exactly `meta deterministic` with no arguments.
- It attaches to ordinary verbs, external verbs, role methods, and role
  implementations.
- Duplicate declarations are rejected.
- A deterministic verb cannot call a signature carrying `filesystem`,
  `network`, or `entropy`.
- A deterministic verb cannot declare, receive, return, construct, or operate
  on `f32` or `f64` values. This includes floating-point values nested inside
  generic and array types.
- The floating-point rule is checked at every call boundary, including
  imported, external, generic, and dynamic role calls. A caller cannot hide a
  floating-point signature behind a wrapper or type substitution.
- `blocking` and `allocation` are not forbidden by this attribute alone;
  they describe execution behavior, not nondeterministic input.
- The rule is transitive through declared effect signatures, imported
  signatures, external declarations, generic substitutions, and dynamic role
  dispatch.
- Role implementations must preserve the deterministic contract declared by
  the role method.
- Effect violations use `E1124` at the forbidden call site. Floating-point
  violations use `E1135`. A role implementation that omits a deterministic
  contract required by its role method uses `E1136`.

`meta deterministic` does not provide a proof that arbitrary native code is
deterministic. Native boundaries must declare their effects explicitly, and
undeclared effects remain semantic errors. The attribute adds no runtime state,
implicit synchronization, or hidden normalization.
