# Common ownership diagnostics

Ownership diagnostics describe a violated source contract. Read the binding,
call site, and preceding move or loan before changing code.

Common situations include:

- using a binding after a `dat` transfer;
- mutating through an `abs` view;
- creating an `ins` loan from an `abs` binding;
- creating two conflicting loans for one owner;
- returning or storing a view beyond its source scope, including a struct,
  `Option` or `Result` that holds a view of a local (`E1018`);
- moving a value inside a loop that runs again (`E1005`, moved in a previous
  iteration), unless the path leaves the loop with `break` or `return`;
- dropping a moved or already-dropped value;
- joining branches with incompatible ownership states;
- omitting a required role at a call site;
- passing an aggregate or resource to scalar-copy syntax;
- reading a struct or an enum after it moved into a binding, field, payload or
  `erg` argument (`E1005`), which `meta copy` and `copy(value: abs x)` resolve
  for plain data.

The stable diagnostic code identifies the compiler category. The source span
identifies the argument, binding, field, or branch that needs attention. Do
not resolve a role diagnostic by copying a resource or weakening a parameter;
change the contract to match the actual ownership operation.
