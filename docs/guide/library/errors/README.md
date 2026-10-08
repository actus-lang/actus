# Errors and results

A fallible public operation should return a typed result such as
`Result[T, E]`. The success value carries the operation's result; the error
variant identifies the failure contract.

Handle a result with `case`:

```actus
case decoded {
    Result[Frame, WireError].Ok(frame) => {
        return inspect(frame: abs frame);
    },
    Result[Frame, WireError].Err(error) => {
        return report(error: abs error);
    },
}
```

Use `?` when the surrounding verb returns a compatible result type. Translate
raw foreign status codes at the private bridge boundary and keep typed errors
in public Actus code.

Document each error variant that callers can receive, including capacity,
invalid input, unsupported profile, corruption, and lifecycle failures.

## Detailed pages

- [Usage guide](usage.md)
- [Standard-library error declarations](../reference/20-standard-library.md)
- [Result and ownership rules](../../ownership/case-and-branch-ownership.md)

An error payload is a value with its own ownership state. When a `case` branch
consumes an error, use `dat` at the branch subject and return or translate the
variant before leaving the branch. Do not inspect raw foreign status integers
outside the private runtime bridge that translates them into the typed error
domain.
