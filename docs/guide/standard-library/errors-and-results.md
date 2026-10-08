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
