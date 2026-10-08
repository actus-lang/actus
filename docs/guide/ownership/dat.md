# `dat`: terminal ownership transfer

`dat` transfers ownership to the callee. The caller cannot use the transferred
binding as its previous owner after the call.

```actus
verb consume(dat resource: Resource) -> Result[Int, ResourceError] {
    return finish(resource: dat resource);
}
```

Use `dat` when the callee becomes responsible for the resource's remaining
lifetime or when the operation intentionally consumes it. The compiler tracks
the move through normal and error paths and schedules cleanup at the new owner.

Do not use `dat` merely to avoid writing an `abs` or `ins` contract. If the
callee only reads or temporarily mutates the value, choose the corresponding
non-terminal role.
