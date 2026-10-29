# `meta deterministic`

`meta deterministic` declares that a verb's observable result must not depend
on external nondeterministic sources. The compiler rejects calls carrying the
`filesystem`, `network`, or `entropy` effects.

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
- `blocking` and `allocation` are not forbidden by this attribute alone;
  they describe execution behavior, not nondeterministic input.
- The rule is transitive through declared effect signatures, imported
  signatures, external declarations, generic substitutions, and dynamic role
  dispatch.
- Role implementations must preserve the deterministic contract declared by
  the role method.
- The diagnostic is `E1124` at the forbidden call site.

`meta deterministic` does not provide a proof that arbitrary native code is
deterministic. Native boundaries must declare their effects explicitly, and
undeclared effects remain semantic errors. The attribute adds no runtime state,
implicit synchronization, or hidden normalization.
