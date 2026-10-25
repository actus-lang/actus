# Temporary resources

`std::fs` exposes secure temporary files and directories through the
`temporary` child module. The caller must provide a validated workspace root
and a bounded ASCII prefix:

```actus
import {
    open std::fs,
    open std::path,
};
```

`temporary_file_open` and `temporary_directory_open` use OS entropy and
atomic create-new/mkdir operations. They reject symlink workspace roots and
retry name collisions only within a bounded runtime budget. The generated
`Path` is returned together with the owned resource, so the caller can record,
publish, or explicitly clean up the exact object created.

Use explicit cleanup when the result must be observable:

```actus
open verb create_workspace(abs root: Path, abs prefix: Buffer)
    -> Result[Int, IoError] {
    erg created = temporary_directory_open(root: abs root, prefix: abs prefix);
    return case dat created {
        Result.Err(_) => Err(IoError.Failed),
        Result.Ok(directory) => {
            return temporary_directory_cleanup(self: ins directory);
        },
    };
}
```

File cleanup closes the handle before removing its path. Directory cleanup
only removes an empty directory; recursive removal is never implicit. Drop
attempts best-effort cleanup for early exits, but Drop cannot report an error,
so callers requiring cleanup diagnostics must use the explicit cleanup verbs.

Temporary creation is blocking filesystem work and must remain outside
inference, impulse forwarding, neurogenesis, and other realtime paths.
