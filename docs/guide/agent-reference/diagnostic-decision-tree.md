# Diagnostic decision tree

Use this order when an Actus command reports an error:

```text
manifest or lockfile?
  -> fix package configuration and rerun check
module or facade?
  -> fix canonical path, export, or import
syntax?
  -> fix source tokens and declaration form
type, role, ownership, or bounds?
  -> fix the source contract and add a focused case
native lowering?
  -> inspect generated reachability, ABI, layout, and target contract
linker or runtime?
  -> inspect objects, runtime providers, libraries, and linker settings
hardware?
  -> reproduce with target-specific evidence
```

Keep the original diagnostic when reporting a compiler boundary. Do not add a
wrapper, duplicate implementation, or undocumented syntax to make the error
disappear.
