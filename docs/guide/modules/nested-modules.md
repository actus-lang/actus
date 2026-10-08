# Nested modules

Nested modules use one canonical facade at every directory boundary:

```text
std/
└── wire/
    ├── wire.act
    └── fragment/
        ├── fragment.act
        ├── codec.act
        └── reassembly.act
```

The parent `wire` facade decides whether `fragment` is public. The child
`fragment` facade decides which of its own siblings are public. External code
uses the parent path exposed by the complete facade chain.

Keep nested modules responsibility-oriented. A child module should own one
cohesive subject such as codec, errors, or reassembly.
