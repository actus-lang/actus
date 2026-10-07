# ADR-0071: Native Lowering for Enum-Valued Result Payloads

- Status: Rejected after reproduction
- Date: 2026-10-06
- Scope: Investigation record for a suspected native lowering defect.

## Context

Twin-e's sensor-to-action boundary initially appeared to expose a native enum
lowering defect. The Actus compiler accepted the source and the strict native
build passed, but the executable terminated while deriving an observation token
identity.

## Decision

The investigation does not authorize a native lowering change. The trap is the
existing checked narrowing behavior for `observation_key(...) as u32`: the
`u64` hash can exceed the `u32` range. Valid enum and nested-facade compiler
fixtures pass independently, so the application must define a bounded token
identity before narrowing. Invalid numeric conversions remain fail-closed.

No compiler fixture is accepted as evidence for an enum bug from this incident.
The downstream fix belongs in Twin-e and must preserve the compiler's checked
conversion contract.

## Acceptance evidence

The investigation is closed as a compiler false positive. The downstream
action-routing tests remain open until Twin-e uses a bounded token identity and
passes strict native execution.
