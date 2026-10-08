# `std::path` ownership and errors

| Value | Role and lifetime |
| --- | --- |
| constructor `Buffer` | `dat`; storage moves into `Path` on success |
| queried `Path` | `abs`; no mutation or transfer |
| builder receiver | `ins`; exclusive call-scoped mutation |
| `join`/`normalize` receiver | `dat`; operation returns or cleans the owner |
| `PathComponent` | borrowed view into the source path |
| `PathComponents` | iterator state with a call-scoped cursor |

## Error variants

- `EmbeddedNull`: a null unit appeared before required termination;
- `InvalidEncoding`: platform units are malformed;
- `InvalidLength`: metadata or unit width is inconsistent;
- `MissingTerminator`: required termination is absent;
- `CapacityExceeded`: existing storage cannot hold a result;
- `UnsupportedPlatform`: representations cannot be combined.

Handle a constructor result before assuming a `Path` exists. Handle a builder
result before relying on the mutation or returned owner. Error mapping from
private runtime statuses occurs inside the facade.

## Component lifetime

A component is a view, not a copied string. If it must survive a path mutation,
copy it into separate caller-owned storage under an explicit conversion
contract. Do not store a component offset without preserving the source path
identity and lifetime.
