# `std::fs` ownership and lifecycle

## Resource rules

- `File` owns a native file handle and requires deterministic cleanup.
- `Path` values are owned data and are passed through `abs` for inspection.
- Read destinations use `ins Buffer`.
- Write sources use `abs Buffer`.
- Options are built as owned values and consumed by operations that open a file.

Do not copy a file handle or store a raw runtime handle in application state.
Use the public `File` value and its typed verbs.

## Seek lifecycle

`SeekFrom` describes the origin for a seek. A successful seek changes the
handle position; a failed seek leaves the failure visible as `IoError`.
Always handle the result before performing a read or write that depends on the
new position.

## Failure paths

If opening fails, there is no handle to close. If reading or writing fails,
retain the handle only when the caller has a documented recovery path; flush
and close it before leaving the owning scope. Atomic operations must not expose
the staging path as the active result after a failed publication.
