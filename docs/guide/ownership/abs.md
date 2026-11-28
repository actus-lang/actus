# `abs`: read-only view

`abs` is a shared, read-only view. It lets a callee inspect an existing value
without taking ownership or scheduling its cleanup.

```actus
verb checksum(abs input: Buffer) -> u16 {
    return crc16(input: abs input);
}
```

The source owner remains responsible for the value. While the view is active,
operations that would invalidate or mutate the owner are restricted.

Use `abs` for inspection, comparisons, encoding from existing data, and public
APIs that do not need to modify or consume the input.

A view can also be held by a struct, an `Option` or a `Result`. Such a value may
be returned, or stored in a field of an `ins` parameter, only when every view in
it comes from an `abs` parameter of the verb, as for a direct `-> abs` return
(ADR-0021). A view of a local or of a value the verb owns is rejected with
`E1018`. The view belongs to the place that holds the value: the source stays
frozen until the end of the scope of the binding, field or element that holds
it, and a source declared in a deeper block than that binding is rejected with
`E1018`, because it would be released first. An `if` or `case` value holds the
views of all its arms, and a variable bound by a `case` pattern holds the views
of the value it matched.

A view owns nothing, so a struct, enum, pack, array or buffer read from one of
its fields or elements cannot be taken by value: `return parser.header;`,
`erg x = parser.header;` or passing `parser.header` on would share the value
silently. The compiler rejects these reads with `E1010` (`E1082` through an
`ins` loan) and names the path. For a type declared
[`meta copy`](../meta-attributes/copy.md) write the duplication,
`copy(value: abs parser.header)`; for any other type keep borrowing it with
`abs` (`abs h = ref parser.header;`, an `abs` argument) or take ownership of the
parser instead. Scalar fields read through a view are unchanged.

An `abs` view cannot be used as an `erg` owner, a `dat` transfer, or an `ins`
loan. If a callee needs one of those meanings, declare that contract explicitly.
