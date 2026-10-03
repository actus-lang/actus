# Gate 23.5 Evidence: Typed Nested Control-Flow Joins

Status: **closed**. Semantic analysis rejects incompatible value-producing
branches, accepts compatible typed branches alongside diverging paths, and
native lowering emits explicit merge arguments or a native terminator for
every reachable branch.

## Implementation evidence

- `if` expressions use an explicit Cranelift merge block parameter and do not
  attach a value edge to branches that return, break, continue, or propagate a
  typed error.
- Nested `return` paths are lowered as native return instructions after the
  branch cleanup plan has run; they are not left as unterminated blocks.
- `case` branch value types are checked in semantic analysis. Diverging block
  branches do not participate in value unification, while every reachable
  value branch must agree with the other value branches.
- Existing ownership, cleanup, indexed-place, nested-call, and loop-control
  paths remain on their ordinary compiler pipeline. No fixture-specific
  lowering or backend shortcut was added.

## Acceptance evidence

The semantic suite covers a typed `case` branch paired with a returning branch
and rejects incompatible branch types. Native execution covers nested value
producing `if` expressions, a nested `if` with a diverging return branch, a
typed `case` with a returning branch, nested indexed places, nested calls,
`?` propagation, and nested case/loop control. Repeated native fixture builds
continue to produce identical object artifacts where deterministic output is
required.

Relevant tests:

- `validates_case_branch_result_types_and_divergence`
- `executes_nested_value_producing_conditionals`
- `executes_nested_if_expression_with_a_diverging_branch`
- `executes_typed_case_branch_with_a_returning_branch`
- `executes_calls_across_nested_statement_blocks`
- `propagates_try_from_a_nested_call_statement`
- `preserves_nested_case_loop_control_after_short_circuit_evaluation`

The repository quality suite, documentation checks, source-limit checks, and
diff validation are required before the gate commit is created.
