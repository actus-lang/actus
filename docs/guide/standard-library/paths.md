# `std::path`

```actus
import std::path;
```
`std::path` provides owned path values, platform classification, validated
storage, component views, predicates, lexical normalization, and builders.

Paths are data values. Lexical normalization does not access the filesystem.
Platform-specific separators and roots are handled by the corresponding public
contracts rather than by manual string replacement.

Component views are non-owning and bounded. Builders mutate owned path storage
through the role required by their operation. Invalid platform forms,
separators, roots, and storage are returned as typed failures.

## Public surface

The facade includes:

- `Path`, `PathPlatform`, and typed `PathError` values;
- constructors from POSIX, Windows UTF-16, and ASCII storage;
- component views and iteration;
- `parent`, `file_name`, `file_stem`, and `extension` queries;
- `is_absolute`, `is_relative`, `has_root`, `starts_with`, and `ends_with`;
- lexical `normalize`;
- `join`, `push`, `reserve_path`, `set_file_name`, and `set_extension`;
- POSIX and Windows root/separator classification.

Path constructors consume caller-provided storage where their declarations use
`dat`. Queries use `abs Path`. Builders either consume and return a new path or
mutate an existing path through `ins`, as shown by each verb signature.
