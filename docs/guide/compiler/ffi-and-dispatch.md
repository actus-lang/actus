# Foreign functions and dispatch

Foreign functions are an explicit boundary between Actus and an external ABI.
Declare them behind the supported unsafe contract, document parameter roles,
layout, status values, and cleanup, and expose a typed Actus facade to normal
application code.

Raw negative status values or pointers should not escape the private bridge.
Translate them into `Result[T, E]` at the boundary. Preserve ownership roles
when buffers, handles, or aggregates cross the call.

`perform` declarations associate an implementation with a type and role
contract. Static dispatch resolves a known implementation. Dynamic dispatch
uses the declared performance and its supported role/vtable contract. A call
must not bypass module visibility or invent an implementation during native
lowering.
