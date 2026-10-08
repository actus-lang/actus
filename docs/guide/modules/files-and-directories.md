# Module files and directories

A single-file module is represented by one `.act` file. A directory module has
a canonical facade whose filename matches the directory:

```text
src/
└── codec/
    ├── codec.act       # canonical facade
    ├── frame.act       # sibling implementation
    └── checksum.act    # sibling implementation
```

The facade declares the public surface. Sibling files share the module's
internal scope and can use declarations from one another without external
import paths.

Use a separate sibling when a responsibility has its own data types, errors,
ownership boundary, or API. Keep the facade focused on module structure and
exports.
