# `std::path` components and predicates

## Borrowed components

`PathComponent` is a view into a source path:

```actus
open struct PathComponent {
    abs source: Path,
    erg offset: Int,
    erg length: Int,
}
```

The component owns no storage. It is valid only while its source path remains
available. The public queries are:

- `parent`: the final parent component, when one exists;
- `file_name`: the final file-name component;
- `file_stem`: the file name without its final extension;
- `extension`: the final extension.

Root-only paths and names without the requested component return `Option.None`.
For example, `archive.tar.gz` has file name `archive.tar.gz`, stem
`archive.tar`, and extension `gz`.

## Iteration

`components(abs self: Path)` creates borrowed iterator state. Each
`next_component(ins iter: PathComponents)` advances the iterator and returns
an `Option[abs PathComponent]`. The iterator stores the source origin and
offset; it does not allocate or copy component bytes.

## Predicates

- `is_absolute` and `is_relative` inspect the selected platform root;
- `has_root` reports whether any root is present;
- `starts_with` and `ends_with` compare complete path components, not raw
  prefixes or suffixes.

Thus `/usr/bin` starts with `/usr`, but it does not start with `/us`.
