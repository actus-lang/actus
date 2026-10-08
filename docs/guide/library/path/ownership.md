# `std::path` ownership and errors

| Value | Role and lifetime |
| --- | --- |
| `Buffer` passed to a constructor | `dat`; ownership moves into `Path` on success. |
| `Path` inspected by a query | `abs`; no mutation or transfer. |
| `Path` receiver of `push` or `set_*` | `ins`; exclusive call-scoped mutation. |
| `Path` receiver of `join` or `normalize` | `dat`; the operation returns or cleans up the owner. |
| `PathComponent` | borrowed `abs` view into the source path. |
| `PathComponents` | borrowed iterator state with an `ins` cursor. |

`PathError` variants are:

- `EmbeddedNull`: a null unit appeared before the terminator;
- `InvalidEncoding`: the platform unit sequence is malformed;
- `InvalidLength`: metadata or unit width is inconsistent;
- `MissingTerminator`: required termination is absent;
- `CapacityExceeded`: existing storage cannot hold the result;
- `UnsupportedPlatform`: the operation cannot combine the representations.

Handle a constructor result before assuming a `Path` exists. Handle a builder
result before relying on the mutation or returned owner. Error mapping from
private runtime status values happens inside the facade.
