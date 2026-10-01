# ADR-0052: Core Control Flow, Named Constants, and Type-Directed Ergonomics

- Status: Proposed
- Date: 2026-10-01
- Scope: Actus language core, semantic analysis, and native control-flow lowering

## Context

Actus deliberately favors explicit ownership, bounded resources, typed results, and deterministic native execution. Those principles must remain visible while ordinary code is written. Recent core work exposed several places where the implementation forces users to write compiler workarounds instead of expressing the intended algorithm:

- protocol and register algorithms have no language-level named constants;
- if is primarily represented as an expression, so statement control flow becomes unnecessarily indirect;
- case branch expressions cannot directly represent a diverging return, break, or continue;
- loop lowering has had fragile merge and exit-block behavior in nested control flow;
- typed integer literals and arithmetic require avoidable temporary bindings;
- public declarations reached through a canonical facade can produce a less useful unresolved-type diagnostic when the export graph is incomplete.

These are language-core issues. They must not be solved by handwritten algorithm duplication, C bridges, hidden conversions, or application-specific compiler exceptions.

## Decision

Actus will add a small, orthogonal core-ergonomics layer while preserving the existing ownership roles, explicit conversions, typed results, facade rules, and one-directional compiler pipeline.

### Named compile-time constants

Actus will support declaration-site constants with explicit types and compile-time values:

~~~actus
const CRC16_POLYNOMIAL: u16 = 0x1021;
const FRAME_HEADER_BYTES: u32 = 12u32;
~~~

Constants are immutable values, not owners. They do not participate in erg, abs, ins, or dat transitions and cannot contain runtime reads, allocation, mutation, or function calls. A constant expression may use other constants and the existing checked primitive operators only when evaluation is deterministic at compile time.

Constants are private by default. An explicitly exported constant must use the same facade visibility rules as every other public declaration and must carry the normal public documentation. The compiler must reject duplicate names, runtime-dependent initializers, overflow, and type-incompatible values at semantic analysis time.

The implementation must preserve the constant's source span and name in diagnostics. Native lowering emits the resolved value directly; no runtime storage or C ABI symbol is created for a constant.

### Statement-position conditionals

Actus will support a statement form that returns no value:

~~~actus
if index >= count {
    break;
}

if ready {
    flush()?;
} else {
    refill()?;
}
~~~

The existing expression form remains distinct:

~~~actus
erg selected = if condition {
    left
} else {
    right
};
~~~

A statement conditional must not silently become a value-producing expression, and an expression conditional must still require compatible branch types or a diverging branch. if conditions remain strictly Bool; Actus does not introduce truthiness or implicit integer-to-boolean conversion.

### Diverging branches and case control flow

return, break, and continue are control-flow statements, not ordinary branch values. The semantic analyzer will classify a branch that cannot complete normally as Diverges and will exclude it from value-type joining. For example, a remaining u32 branch may determine the type of an expression when the other branch returns or breaks.

case remains the pattern-matching expression and keeps its existing exhaustiveness and ownership rules. Its block branches may contain validated statements, including diverging control flow, through the same statement grammar used by loops and conditionals. A branch that diverges must not create a merge value or re-enter the case merge block. This extends control-flow expressiveness without weakening pattern coverage or ownership validation.

### Deterministic loop CFG lowering

The native backend will lower each loop with an explicit loop-target frame:

- a header block for loop-carried values;
- a body block;
- a continue target that performs the required back-edge cleanup;
- an exit target that receives the loop result state;
- a merge contract that is completed exactly once.

Nested case, if, and block scopes must resolve break and continue to the nearest active loop frame. Diverging branches must terminate their current Cranelift block before another block is selected. The code generator may not repair invalid semantic control flow or infer ownership after a branch.

### Type-directed integer ergonomics without implicit conversion

Typed literals remain explicit and are extended consistently across arithmetic, comparison, indexing, and assignment:

~~~actus
erg index: u32 = 0u32;
index += 1u32;
~~~

An unsuffixed integer literal may receive its type from a declared destination or a checked operator context when the value fits. A suffixed literal has the declared primitive type and is checked at compile time. Binary operations still require compatible operand types; u32 is not implicitly converted to Int, and a conversion still requires expr as Type.

The analyzer must preserve one evaluation of every assignable place. Native compound assignment must calculate the address once, load once, perform the typed operation, and store once. This rule applies equally to identifiers, fields, arrays, and buffers.

### Facade-aware public type resolution

Type lookup must resolve public declarations through the canonical module facade before reporting an unknown type. The resolver will distinguish:

1. a declaration that exists and is exported;
2. a declaration that exists only in a private sibling;
3. a missing declaration;
4. a malformed or stale facade graph.

Diagnostics must report the actionable module or facade boundary and must not depend on code-generation state. Private declarations remain inaccessible; this decision improves resolution and diagnostics without widening visibility.

## Invariants

1. No implicit ownership conversion is introduced by these features.
2. No implicit numeric conversion is introduced; explicit casts remain the only conversion mechanism between incompatible primitive types.
3. Constant evaluation is bounded, deterministic, allocation-free, and free of runtime or foreign-function calls.
4. Statement conditionals return Void; expression conditionals produce one joined value or diverge.
5. Every break and continue resolves to one lexical loop target.
6. Every native basic block is terminated exactly once before control switches to another block.
7. A diverging branch contributes no value and no invalid ownership state to a merge point.
8. Canonical facades remain the only public module boundary.
9. Parser, semantic, and codegen modules remain independent in the order lexer -> parser -> AST -> semantic -> codegen.
10. No feature requires a temporary C bridge or application-specific lowering.

## Consequences

### Positive consequences

- Protocol, embedded, and register code can name its invariants once instead of repeating numeric literals.
- Ordinary early-exit and loop code becomes readable without weakening Actus's explicit type and ownership model.
- Branch and loop behavior becomes deterministic for both the native backend and tooling.
- Better facade diagnostics reduce false unresolved-type failures while preserving encapsulation.
- The same AST and semantic facts can drive formatter, LSP, diagnostics, and Cranelift lowering without backend-specific repairs.

### Costs and risks

- Constants add a compile-time evaluation phase and require cycle detection.
- Statement and expression conditionals require separate AST and semantic paths, even though they share condition and branch validation.
- Loop lowering needs regression coverage for nested scopes, cleanup, and early exits on every supported target profile.
- Type-directed literals must be specified narrowly to avoid accidental inference or hidden conversions.

## Non-goals

- This ADR does not remove case or replace pattern matching with if.
- This ADR does not add truthy/falsy conditions.
- This ADR does not add implicit casts, dynamic numeric promotion, or a general compile-time programming language.
- This ADR does not change ownership roles, public facade visibility, or C ABI status contracts.
- This ADR does not implement std::wire or any application protocol.

## Acceptance criteria

The ADR is implemented only when evidence exists for:

- accepted and rejected named constants, including overflow, cycles, and runtime-dependent initializers;
- statement if, expression if, nested else if, and diverging branches;
- case branches containing valid return, break, and continue paths;
- nested loops with cleanup and deterministic nearest-target resolution;
- typed literal inference, explicit casts, compound assignment, and single-evaluation lvalue behavior;
- public, private, missing, and malformed facade type-resolution diagnostics;
- formatter and LSP support for every new syntax form;
- native execution tests for both successful paths and runtime traps;
- parser, semantic, codegen, source-limit, and full repository quality checks.

