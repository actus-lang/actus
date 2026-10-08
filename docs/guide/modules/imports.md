# Imports

Import a public declaration through its canonical module path. The import must
resolve through each parent facade.

```actus
import std::wire::WireHeader;
import std::wire::wire_frame_encode;
```

The exact declaration names come from the public facade. A direct path to a
private sibling is not a valid replacement for a missing export.

If an import fails, check the module directory, canonical facade filename,
facade export, declaration visibility, and package source root in that order.
Do not duplicate the implementation or add an unrelated wrapper to hide a
facade error.
