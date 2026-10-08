# Common ownership diagnostics

Ownership diagnostics describe a violated source contract. Read the binding,
call site, and preceding move or loan before changing code.

Common situations include:

- using a binding after a `dat` transfer;
- mutating through an `abs` view;
- creating an `ins` loan from an `abs` binding;
- creating two conflicting loans for one owner;
- returning or storing a view beyond its source scope;
- dropping a moved or already-dropped value;
- joining branches with incompatible ownership states;
- omitting a required role at a call site;
- passing an aggregate or resource to scalar-copy syntax.

The stable diagnostic code identifies the compiler category. The source span
identifies the argument, binding, field, or branch that needs attention. Do
not resolve a role diagnostic by copying a resource or weakening a parameter;
change the contract to match the actual ownership operation.
