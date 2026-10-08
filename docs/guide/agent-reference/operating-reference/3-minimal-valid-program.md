# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 3. Minimal valid program

Hosted Alpha applications use a `main` entry verb. The current hosted entry
contract is an integer-returning main for application execution, although a
`Void` verb is also a valid language declaration and is useful for helpers and
runtime-profile examples.

```act
verb main() -> Int {
    return 0;
}
```

A source file may contain multiple verbs. The configured package entry is
selected by `Actus.toml`; hosted executable workflows currently require the
configured entry name to be `main`.

A `Void` verb has no return expression:

```act
verb initialize() -> Void {
    return;
}
```

For a statement-only helper, omitting the return type is also used by existing
source and is represented as the language's no-value return form. When adding
new code, prefer an explicit `-> Void` when the public contract should be
obvious.
