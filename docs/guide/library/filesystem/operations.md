# Path operations and atomic publication

## Direct filesystem operations

`remove_file`, `rename`, `copy_file`, `create_dir`, and `remove_dir` borrow
validated `Path` values and perform one hosted side effect. They return a
status or copied byte count through `Result`.

`canonicalize` is the separate filesystem-identity operation. It resolves all
path components and symbolic links through the host, returning a new absolute
`Path`; it does not mutate the lexical input. Missing components are errors.
Use `std::path::normalize` when only lexical cleanup is required and no
filesystem lookup is allowed.

`same_file` is identity-based rather than lexical. It follows links and
compares the native identity pair supplied by the target (POSIX device/inode
or the Windows volume/file index). Targets without a stable identity return
`IoError.Unsupported`.

## Directory handle lifecycle

`read_dir` returns one owned `Directory` handle. `next_dir_entry` streams one
owned entry `Path` at a time and returns `Ok(Option.None)` at end of
directory. Repeating `next_dir_entry` after EOF continues to return
`Ok(Option.None)`; EOF is not a failure. `close_dir(ins directory)` releases
the host iterator and marks the owner closed, so its Drop cleanup is
idempotent.

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
