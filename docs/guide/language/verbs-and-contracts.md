# Verbs and contracts

A `verb` is an Actus operation. Its declaration describes the parameter roles,
parameter types, return type, and body behavior.

## Basic verb

```actus
verb increment(erg value: u32) -> u32 {
    value += 1u32;
    return value;
}
```

The role is part of the contract. The caller must provide an argument with a
compatible ownership meaning.

## Read-only inputs

Use `abs` when the verb only inspects a value:

```actus
verb is_zero(abs value: u32) -> Bool {
    return value == 0u32;
}
```

Use `ins` when the verb needs exclusive temporary mutation of caller-owned
storage. Use `dat` when the call intentionally transfers terminal ownership.
See the ownership category for the complete rules.

## Visibility

Declarations are private unless exposed through the module's canonical
facade. Public verbs require documentation and a stable source contract.

## Calls

Keep the role visible at the call site when the ownership meaning matters:

```actus
erg result: u32 = add(left: abs first, right: abs second);
```

A call must satisfy the callee's parameter type and role. The compiler checks
nested calls, generic calls, return values, cleanup, and failure propagation.

## Methods and dispatch

A `perform` declaration associates an implementation with a type and role
contract. Method-like calls use the receiver and declared parameters. Static
and dynamic dispatch have separate contracts; dynamic calls require a matching
performance implementation and explicit supported roles.

Do not use a wrapper to hide a role mismatch or a private module declaration.
Fix the declaration, facade, or call boundary that owns the contract.

## Structured documentation

Public verbs should document:

- purpose;
- input names, types, and roles;
- return value and typed failures;
- ownership transfer or restoration;
- cleanup and side effects;
- allocation and I/O behavior;
- bounds and invariants.
