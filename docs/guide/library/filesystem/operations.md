# Path operations and atomic publication

## Direct filesystem operations

`remove_file`, `rename`, `copy_file`, `create_dir`, and `remove_dir` borrow
validated `Path` values and perform one hosted side effect. They return a
status or copied byte count through `Result`.

`remove_dir` is for an empty directory according to the host provider. A
missing, protected, non-empty, or otherwise rejected target is reported as
`IoError.Failed`.

## One-shot reads

`read_to_bytes` opens a path, reads all available bytes into a newly owned
result buffer, and closes the temporary handle through normal cleanup. It is
appropriate for bounded files where the caller accepts the resulting buffer
size.

`read_to_string` follows the same lifecycle and then validates the bytes as
UTF-8. Invalid bytes return `IoError.InvalidData`; they are not returned as a
partially valid text value. The result type is still `Buffer` because Actus's
current filesystem facade represents validated text as length-delimited bytes.

## Direct write

`write_file` creates or truncates the destination and writes the borrowed
contents. It reports the provider's byte count and typed failure. It does not
stage the destination, so callers needing crash-safe publication should use
`write_file_atomic`.

## Atomic publication

`write_file_atomic` uses this sequence:

1. open or create the caller-selected `staging` path;
2. write the complete contents;
3. verify the written count matches the source buffer length;
4. flush the file;
5. rename `staging` to the final `path`;
6. remove staging and return the original typed error if any step fails.

The staging path should be on the same filesystem as the final path so rename
has the provider's intended publication semantics. The final path is not
published until the staged write and flush succeed.

The operation consumes `staging` because every failure path must have one clear
owner for cleanup. It does not allocate a hidden path or buffer.

## Failure behavior

A write, short count, flush, or rename failure is returned as a typed error.
Failed publication attempts remove the staging path when possible and preserve
the original failure. Cleanup is best-effort side effect; it must not turn a
failed write into a successful publication.

## Hot-path boundary

All operations in this page can block and can mutate external state. Keep them
behind explicit save, commit, or service calls. Inference, propagation,
neurogenesis, and other hot paths must not call these verbs synchronously.
