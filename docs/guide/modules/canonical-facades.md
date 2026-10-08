# Canonical facades

The canonical facade is the source file that has the same name as its module
directory. It is the only external gateway to that directory module.

A facade can expose sibling declarations with extensionless `open` declarations:

```actus
open frame;
open checksum;
```

A declaration that is not exposed remains private to the module's internal
scope. The compiler resolves sibling discovery and facade exports
deterministically and rejects duplicate or unknown exports.

When a public child module is needed, expose that child through the parent
facade. External code then uses the parent path and does not depend on the
child's internal filesystem location.
