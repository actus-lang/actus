# `meta stack_limit(...)`

`meta stack_limit` declares a target-specific machine-stack proof obligation
for a verb or another callable boundary. It is not a source-size estimate and
it does not insert a runtime stack check.

```actus
meta stack_limit(bytes: 2048u32)
verb control_step(ins state: ControlState) -> Int {
    return 0;
}
```

The only accepted argument is a positive `u32` literal named `bytes`. Unknown
arguments, duplicate fields, zero, unsuffixed integers, wrong integer widths,
duplicate attributes, and unsupported placements are compile-time errors.

The attribute may be attached to an ordinary verb, a `perform` implementation,
a role-method declaration, or an `unsafe extern` declaration. It is not a
file-prelude attribute, a type attribute, or a replacement for ownership
roles. `meta limitless` cannot disable it.

## Proof contract

Once machine-level enforcement is implemented, the selected backend must prove
that every reachable execution stays within the declared limit. The proof uses
the backend's final frame report, a closed machine call graph, target-specific
call-frame rules, helper metadata, and checked max-plus arithmetic. Unknown
frames, unresolved calls, open indirect dispatch, and reachable recursion are
rejected; they are never treated as using zero bytes.

The initial supported proof boundary is restricted to target/object-format
contracts with complete frame evidence. LLVM ELF targets are the first planned
implementation boundary. Mach-O, COFF, and Cranelift require their own proven
frame-report contracts before they can accept this attribute.

Native claims must describe the complete transitive stack use of the native
callee. A claim is trusted as `declared` unless a compatible native frame
report upgrades it to `verified`. Under `--strict`, native claims must be
`verified` or listed as a reviewed conformance exception.

## Relationship to `bounded`

`meta bounded(max_stack_bytes: ...)` remains a source-level budget described by
ADR-0095. It does not prove the machine frame. `stack_limit` and `bounded` may
coexist, and their diagnostics must remain distinct. No automatic deprecation
or migration is implied by `stack_limit`.

## Current implementation boundary

The current frontend accepts and formats the typed metadata contract and
provides parser and negative-fixture coverage. Machine frame extraction,
interprocedural stack analysis, object publication rules, and native frame
verification are later implementation work defined by ADR-0103.
