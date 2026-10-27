# Path operations and atomic publication

## Direct filesystem operations

Metadata snapshots contain the file kind, byte size, readonly flag, and
signed Unix-second timestamps for modification, access, and creation when the
provider supplies them. Each timestamp has a status: `METADATA_VALUE_AVAILABLE`,
`METADATA_VALUE_UNSUPPORTED`, or `METADATA_VALUE_UNAVAILABLE`. Platform
identity is represented by two opaque scalar components plus
`identity_kind`/`identity_status`; POSIX uses device/inode and Windows uses
volume/file-index. Portable callers must inspect the status before using a
timestamp or identity component.

Use `metadata_detailed` or `symlink_metadata_detailed` when the operation
failure itself must be preserved. Their `MetadataError` result distinguishes
`NotFound`, `PermissionDenied`, `Unsupported`, and generic `Failed`; the
compatibility `metadata` and `symlink_metadata` verbs continue to expose the
existing `IoError` surface.

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
owned `DirectoryEntry` at a time and returns `Ok(Option.None)` at end of
directory; it never materializes the complete directory as an array. Native
directory order is unspecified and must not be used as reproducible input for
builds or deterministic consumers. Each entry owns its name and relative path
component and carries a
copied `Metadata` snapshot plus a `DirectoryEntryKind` classification. The
native bridge uses link metadata, so `Symlink` describes the directory entry
itself and does not follow its target; a broken link remains classifiable as a
symlink when the host exposes it.

Use `read_dir_ordered(path, DirectoryOrder.Native)` to state that native order
is intentional, or `read_dir_ordered(path, DirectoryOrder.Lexicographic)` for
bounded reproducible order. Lexicographic mode snapshots and sorts at most
`DIRECTORY_MAX_ORDERED_ENTRIES` entries and rejects a larger directory or a
name exceeding the entry storage bound. Comparison is bytewise over the
platform's canonical name representation, is case-sensitive, and performs no
Unicode normalization; callers needing another policy must normalize names
before creating a separate bounded consumer contract.

Use `directory_entry_is_file`, `directory_entry_is_dir`, and
`directory_entry_is_symlink` for boolean classification, or
`directory_entry_file_type` when the complete `DirectoryEntryKind` value is
needed. These helpers borrow the entry immutably and do not inspect the host
filesystem again.

`read_dir_filtered` applies a transferred `DirectoryFilter` before returning
an entry. `regular_files_only` takes precedence over the kind flags;
`include_directories` and `include_symlinks` control those kinds, while
`include_hidden` means leading-dot names rather than host-specific hidden
attributes. An optional extension is compared bytewise and case-sensitively.
Rejected entries are consumed and never returned, and filtering does not
materialize an unbounded collection.

Repeating `next_dir_entry` after EOF continues to return `Ok(Option.None)`;
EOF is not a failure. `close_dir(ins directory)` releases the host iterator
and marks the owner closed, so its Drop cleanup is idempotent. A subsequent
read or close on that closed owner returns a typed failure; a moved owner is
rejected statically by the ownership checker.
`Directory` implements `Drop`: leaving scope with a live handle performs a
best-effort close, so early returns and branch exits do not silently retain the
host iterator. The compiler binds this cleanup to the owner value live in each
control-flow block.

## Recursive walking

`walk` is a separate depth-first API; it never changes `read_dir` into a
recursive operation. `WalkOptions.max_depth` is zero-based: depth zero emits
the requested root lifecycle, and a child is visited only when its depth is at
most the configured limit. `next_walk_event` streams owned events in this
order for each directory: `EnterDirectory`, child events, then
`LeaveDirectory`. A root file or non-followed root symlink emits one terminal
event instead of directory lifecycle events.

Set `follow_symlinks` explicitly. When false, links emit `Symlink` without a
target lookup. When true, directory identities are canonicalized and tracked;
re-entering an identity is a cycle. `WalkErrorPolicy.FailFast` returns the
typed failure from `next_walk_event`, while `EmitError` emits an `Error` event,
skips that entry, and continues with later siblings. Missing targets and
unsupported operations are never silently ignored. A host permission failure
is preserved as `IoError.PermissionDenied` in either policy rather than being
collapsed into `IoError.Failed`.

`DirectoryOrder.Native` preserves host iterator order and is not reproducible.
`DirectoryOrder.Lexicographic` sorts each directory level by the canonical
platform byte or code-unit name representation. The hosted walker bounds
depth, visited directory identities, entries per directory, and entry-name
length; exceeding a bound is a typed failure. `Ok(Option.None)` is clean end
of stream. `close_walker(dat walker)` releases all retained frames, and the
`Walker` Drop performance provides best-effort cleanup on early exits.

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

`write_file_atomic_synced` adds an explicit parent-directory sync after the
rename. Unix supports this stronger directory-entry ordering; unsupported
targets return `IoError.Unsupported`. A failure at this post-rename step does
not undo publication, so callers must report durability as uncertain.

The operation consumes `staging` because every failure path must have one clear
owner for cleanup. It does not allocate a hidden path or buffer.

`sync_parent_directory` exposes the directory-sync operation independently for
callers that publish through another validated rename path. It borrows the
path, performs no publication, and has no hidden allocation.

## Coordinated compare-before-publish

`publish_file_coordinated` is the safe compare-before-publish surface for
cooperating writers. The caller obtains an expected `Metadata` snapshot (or
uses `metadata_absent()` for an expected missing destination), chooses one
lock path shared by all writers, and supplies a staged path:

1. the runtime opens and exclusively locks the coordinator path;
2. it compares the destination's native identity under that lock;
3. it returns `Conflict` if the identity is stale or the destination appeared;
4. it atomically replaces the destination while still holding the lock.

The lock protects only callers that use the same coordinator path. It does not
claim to serialize an unrelated process that writes the destination directly.
Targets without the required locking or atomic replacement return
`IoError.Unsupported`; the implementation never degrades to an unlocked
metadata check followed by rename.

## Failure behavior

A write, short count, flush, or rename failure is returned as a typed error.
Failed publication attempts remove the staging path when possible and preserve
the original failure. Cleanup is best-effort side effect; it must not turn a
failed write into a successful publication.

## Crash recovery

The atomic publication point is the successful rename, not the initial write
or flush. Before rename succeeds, the active destination is not published by
the atomic helper; write, short-write, flush, close, and rename failures
return typed errors and attempt best-effort staging cleanup. After rename
succeeds, the new namespace entry is published even if a later parent-directory
sync fails.

`write_file_atomic` provides atomic namespace publication without a crash-
durability promise. A crash before rename may leave the previous destination
and stale staging storage. `write_file_atomic_synced` performs the explicit
Unix parent-directory sync after rename; successful completion confirms that
ordering operation, not immunity from every storage-device or power failure.
The library does not scan or delete stale staging files implicitly. Callers
must choose a private staging namespace and define recovery and retention.

The strict publication fixture covers deterministic failed-rename cleanup and
replacement of an existing stale staging path. It does not claim to simulate
truncated writes, provider flush faults, or process interruption; those cases
require a dedicated test-only fault-injection seam and remain outside the
production API.

## Hot-path boundary

All operations in this page can block and can mutate external state. Keep them
behind explicit save, commit, or service calls. Inference, propagation,
neurogenesis, and other hot paths must not call these verbs synchronously.
