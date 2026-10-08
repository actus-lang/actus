# First Actus program

## Create a project

Create or initialize a package with the Actus CLI. The package contains a
manifest, a source root, and an entry source file.

A minimal source file is:

```actus
verb main() -> Int {
    return 0;
}
```

Use the package commands described in [project setup](../workflow/project-setup.md)
to create the manifest and source layout.

## Add a value

Actus uses explicit types and typed integer literals:

```actus
verb main() -> Int {
    erg answer: u32 = 42u32;
    return answer as Int;
}
```

`answer` is an owned mutable local because it has the `erg` role. The `as`
cast is explicit because `u32` and `Int` are different types.

## Add a checked operation

A public operation that can fail returns a typed result:

```actus
enum ParseError {
    Empty,
    Invalid,
}

verb parse_code(abs input: u32) -> Result[u32, ParseError] {
    if input == 0u32 {
        return Result[u32, ParseError].Err(ParseError.Empty);
    }
    return Result[u32, ParseError].Ok(input);
}
```

Use `case` to handle the result:

```actus
verb main() -> Int {
    erg result: Result[u32, ParseError] = parse_code(input: abs 7u32);
    case result {
        Result[u32, ParseError].Ok(value) => {
            return value as Int;
        },
        Result[u32, ParseError].Err(error) => {
            return 1;
        },
    }
}
```

The exact ownership roles for aggregate values depend on the declaration and
call contract. Read the ownership page before moving or lending a result.

## Check the program

Use the package's check command before building an executable. The check phase
parses source, resolves modules, checks types and ownership, and reports
structured diagnostics.

Then use the project test and build commands described in
[check, test, and build](../workflow/check-test-and-build.md).
