# `meta limitless(...)`

`meta limitless("scope")` records an explicit exception to Actus source-size
conformance limits. It does not disable language safety or semantic checks.

The supported scopes are:

- `"verb"`: applies to the immediately following verb;
- `"file"`: applies to the source file metadata prelude and its declarations.

Verb scope:

```actus
meta limitless("verb")
verb generated_decoder() -> Int {
    return 0;
}
```

File scope:

```actus
meta limitless("file")
"""Reviewed generated compatibility surface."""
```

## Policy

The normal source limits remain enabled by default. Use an exception only when
decomposition would make a generated, compatibility, or deliberately cohesive
boundary less clear. Every exception should have a documented reason and
remain visible in review and conformance reports.

`meta limitless("verb")` affects only one immediately following verb.
`meta limitless("file")` is a file-level prelude decision. Neither form
disables parser limits, recursion limits, memory bounds, ownership checks,
unsafe-bridge checks, documentation requirements, ABI validation, or runtime
safety contracts.

## Invalid use

Unknown scopes, duplicate directives, misplaced directives, and directives
inside a verb body are rejected. An arbitrary comment or unknown metadata name
cannot approve a source-limit exception.

The package-wide `source_limits = "limitless"` policy in `Actus.toml` is a
separate, explicit project decision. It should not be used as a substitute for
a narrowly scoped source exception.

