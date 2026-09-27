# ADR-0029: Uniform Method Call Syntax (UFCS)

- Status: Accepted
- Date: 2026-09-26
- Scope: receiver syntax and role-qualified method calls

## Context

Actus currently expresses operations as verbs with explicit named arguments.
That form makes ownership roles visible, but repeated receiver labels make
stream, buffer, and resource APIs harder to scan. A uniform receiver spelling
can improve ergonomics without hiding ownership or dispatch.

## Decision

Actus will support `receiver.verb(arguments...)` as syntax sugar for a verb
call whose first parameter is the receiver. The desugared call retains the
declared role and the call-site role requirement:

```actus
buffer.append(item: erg item);
```

is equivalent to the corresponding explicit verb call with `buffer` as its
first argument. The receiver's role is never inferred as a permission to
mutate. An `ins` method must still expose the exclusive loan at the call
boundary, and an `abs` or `dat` method must preserve its existing ownership
contract.

Static performances resolve to direct calls after desugaring. Dynamic role
dispatch continues to use the existing explicit dynamic ABI. UFCS does not
introduce vtables, implicit borrowing, or hidden allocation.

## Constraints and verification

- [x] Add parser support for receiver expressions and named arguments.
- [x] Desugar before semantic ownership checking.
- [x] Preserve explicit `erg`, `abs`, `dat`, and `ins` call-site validation.
- [x] Resolve static performances through direct monomorphized calls.
- [x] Add diagnostics for receiver/first-parameter mismatches.
- [x] Cover chained calls without changing evaluation order.

The explicit verb form remains canonical in generated documentation and ABI
contracts; UFCS is an ergonomic source spelling.

## ABI and evaluation-order contract

UFCS does not create a second calling convention. After desugaring, the call
uses the same direct static ABI as the canonical verb form, including hidden
caller-allocated `sret` storage for struct returns and the registered layouts
for wide integers, floating-point values, and `Void`.

The receiver expression is evaluated first, followed by explicit arguments from
left to right. The receiver is passed with the role required by the first verb
parameter; no implicit borrow, dereference, conversion, allocation, or status
code translation is inserted by UFCS.
