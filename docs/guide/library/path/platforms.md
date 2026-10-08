# `std::path` platforms and representation

`PathPlatform` identifies the representation used by a path. POSIX and
Windows root classification are separate APIs because separators and roots do
not have the same grammar.

Use `posix_root` and `windows_root` after validating the path. Use the
platform-specific separator predicates when processing components. Do not
replace separators manually before validation.

`PathComponent` values are views into the original path. Copy the component
into separate owned storage only when it must survive the path owner.

Path errors cover invalid storage, invalid roots, invalid separators, invalid
platform data, and capacity failures. Handle them before using a returned
`Path` or component.
