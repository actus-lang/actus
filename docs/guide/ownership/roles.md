# Ownership roles

Roles appear both in declarations and, when needed, at call sites.

```actus
verb inspect(abs value: u32) -> Bool {
    return value == 0u32;
}

verb update(erg value: u32) -> u32 {
    value += 1u32;
    return value;
}
```

The callee's role describes what it may do. The caller's argument role states
how the value crosses that call boundary.

```actus
erg count: u32 = 0u32;
count = update(value: erg count);
inspect(value: abs count);
```

For simple scalar calls, some static calls can infer a compatible `abs` or
`ins` role from the binding. Use an explicit role when the ownership intent
should be visible or when the call is generic, external, dynamic, aggregate,
or otherwise outside inference.

Roles do not mean mutability alone. `erg` owns and may mutate; `ins` lends
exclusive mutation temporarily; `abs` views without ownership; `dat` transfers
ownership and ends the caller's use of that resource.
