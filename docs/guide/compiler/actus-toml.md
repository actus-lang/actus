# `Actus.toml`

`Actus.toml` is the package manifest. It identifies the package, language
edition, source root, entry, runtime profile, target, and build settings.

A minimal application manifest has the package name, version, edition, source
root, and entry contract needed by the selected workflow. Runtime modules and
target settings are declared explicitly when the package uses them.

Keep manifest settings aligned with source imports. A package cannot import a
runtime module that its selected profile does not provide. Unknown or
conflicting manifest fields are reported before source emission.

Use the compiler's manifest and configuration diagnostics rather than adding a
source-level workaround for a package setting error.
