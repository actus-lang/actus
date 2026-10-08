# Lockfiles

`Actus.lock` records resolved package and runtime dependency state. It keeps
repeated package operations deterministic and lets the compiler check whether
the manifest and resolved state agree.

Use the lock command described by the project workflow to synchronize and check
the lockfile. Review lockfile changes separately from language or runtime
changes. Do not hand-edit a generated lock value to hide a dependency mismatch.
