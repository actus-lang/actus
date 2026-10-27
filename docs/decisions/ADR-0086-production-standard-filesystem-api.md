# ADR-0086: Production Standard Filesystem API

- Status: Accepted
- Date: 2026-10-22
- Scope: Hosted `std::fs`, `std::path`, native filesystem bridges, and
  applications that need bounded file and directory access

## Context

Actus already exposes the first hosted file operations, but the current
surface is not sufficient for a production data pipeline. In particular, an
application can read a named file but cannot enumerate a directory through a
typed, ownership-safe, deterministic API. Downstream data pipelines need to
process many input files from one configured directory without storing a
per-file list in configuration or hardcoding file names.

The filesystem library must also be useful to other Actus applications. It
therefore needs an architecture for paths, file handles, directory iteration,
metadata, publication, security, limits, and backend parity. A single
`read_dir` verb without lifecycle and failure contracts would only move the
problem into application code.

## Decision

Actus will provide a production hosted filesystem capability through the
canonical `std::fs` and `std::path` facades. The public surface will be
streaming, typed, ownership-aware, position-independent, and explicit about
blocking behavior. Filesystem handles and paths are runtime resources; they
are not serializable Actus state.

The implementation is staged by Phase 46. The initial implementation must
establish the directory-handle and entry contract before higher-level walking,
publication, or application integrations are added.

## Public API contract

### Paths

The API reuses the existing `Path` type and completes the following operations:

```act
path_from_ascii(...)
path_from_utf8(...)
join(...)
parent(...)
file_name(...)
file_stem(...)
extension(...)
components(...)
normalize(...)
canonicalize(...)
is_absolute(...)
is_relative(...)
starts_with(...)
ends_with(...)
relative_to(...)
same_location(...)
```

Lexical normalization must not silently perform filesystem I/O. `canonicalize`
is the explicit operation that resolves filesystem identity and can fail when
components do not exist. The API defines root, separator, dot, dot-dot,
embedded-NUL, invalid-encoding, reserved-name, and platform-specific path
behavior for POSIX and Windows.

### Files and open options

The file API contains explicit lifecycle and complete-I/O operations:

```act
file_open(...)
file_create(...)
file_close(...)
file_read(...)
file_read_exact(...)
file_read_to_bytes(...)
file_write(...)
file_write_all(...)
file_flush(...)
file_sync_data(...)
file_sync_all(...)
file_seek_start(...)
file_seek_current(...)
file_seek_end(...)
file_position(...)
file_length(...)
file_truncate(...)
file_clone_handle(...)
```

`OpenOptions` explicitly controls `read`, `write`, `append`, `truncate`,
`create`, `create_new`, `close_on_exec`, and supported sync behavior. Short
reads and short writes are reported or completed by the corresponding verb;
they are never silently treated as complete-buffer success.

`close_on_exec` is applied as part of native handle creation, not by a later
best-effort flag mutation. POSIX targets use `O_CLOEXEC`; Windows relies on
the standard non-inheritable handle created by `OpenOptions`. Targets without
one of these guarantees reject a requested `close_on_exec` open with the
typed failure surface. The option is therefore a security contract, not a
documentation-only hint, and the platform-specific fixtures select their
assertions with `meta target("unix")` and `meta target("windows")`.

### Directory handles and entries

Directory enumeration is streaming. The API must not return an unbounded
array or hide an unbounded allocation in the ordinary iterator:

```act
open enum DirectoryEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

open struct DirectoryEntry {
    erg name: Path,
    erg kind: DirectoryEntryKind,
    erg metadata: Metadata,
}

open struct Directory {
    erg handle: Int,
}

open verb read_dir(abs path: Path) -> Result[Directory, IoError];
open verb next_dir_entry(
    ins directory: Directory
) -> Result[Option[DirectoryEntry], IoError];
open verb close_dir(
    dat directory: Directory
) -> Result[Int, IoError];
open verb rewind_dir(
    ins directory: Directory
) -> Result[Int, IoError];
```

`read_dir` opens one directory. `next_dir_entry` returns one entry at a time;
`Ok(Option.None)` is the normal end-of-directory result, not an error.
`close_dir` consumes the handle. Reads after EOF, reads after close, double
close, and use of a moved handle are typed failures or compiler-rejected
ownership violations as appropriate.

The basic iterator is non-recursive and returns names relative to the opened
directory. Applications explicitly join those names to their owned base path.
Symlink following is never implicit.

### Ordering and filtering

Native directory order is unspecified. Reproducible consumers can request an
explicit order:

```act
open enum DirectoryOrder {
    Native,
    Lexicographic,
}

open struct DirectoryFilter {
    erg regular_files_only: Bool,
    erg include_directories: Bool,
    erg include_symlinks: Bool,
    erg include_hidden: Bool,
    erg extension: Option[Path],
}

open verb read_dir_filtered(
    abs path: Path,
    erg order: DirectoryOrder,
    dat filter: DirectoryFilter
) -> Result[Directory, IoError];
```

Lexicographic ordering is defined over the canonical path byte representation.
Case sensitivity, Unicode normalization, and invalid-name behavior are part
of the target profile. The current profile is case-sensitive and performs no
Unicode normalization. Filtering is declarative: regular-file-only mode,
directory and symlink inclusion, leading-dot hidden-name inclusion, and an
optional exact extension are evaluated before an entry is returned. Ordering
must be stable across repeated runs and both native backends. If a target
cannot provide the requested order within its declared resource limits, it
returns a typed error instead of silently using native order.

### Metadata

The metadata surface includes:

```act
metadata(path)
symlink_metadata(path)
file_type(path)
file_size(path)
is_file(path)
is_dir(path)
is_symlink(path)
is_readonly(path)
modified_time(path)
accessed_time(path)
created_time(path)
```

`Metadata` distinguishes regular files, directories, symlinks, and other
objects; reports size, readonly state, supported timestamps, and optional
platform identity. `metadata` may follow links; `symlink_metadata` describes
the link itself. Missing, inaccessible, unsupported, and raced metadata are
distinct outcomes where the target can distinguish them.

The portable snapshot stores modification, access, and creation timestamps as
signed Unix seconds. Each timestamp has a status with `Available`,
`Unsupported`, and `Unavailable` meanings; a timestamp value is consumed only
when its status is `Available`. Platform identity is two opaque unsigned
components plus an identity kind: POSIX maps them to device/inode and Windows
maps them to volume/file index. An identity status distinguishes an available
pair from a provider that cannot expose one. The snapshot owns no host handle
and performs no follow-up filesystem lookup.

Applications that need the operation failure cause use
`metadata_detailed(path)` or `symlink_metadata_detailed(path)`. These return a
typed metadata error distinguishing not-found, permission-denied, unsupported,
and generic provider failure. The compatibility metadata verbs retain the
existing `IoError` result surface and intentionally collapse those causes.
The recursive walker does not use that compatibility collapse: a native
permission failure is exposed as `IoError.PermissionDenied`.

### Recursive walking

Recursion is a separate API and is never hidden inside `read_dir`:

```act
open struct WalkOptions {
    erg max_depth: u32,
    erg follow_symlinks: Bool,
    erg include_hidden: Bool,
    erg order: DirectoryOrder,
    erg error_policy: WalkErrorPolicy,
}

open enum WalkEventKind {
    EnterDirectory,
    File,
    Symlink,
    LeaveDirectory,
    Error,
}

open verb walk(abs path: Path, abs options: WalkOptions) -> Result[Walker, IoError];
open verb next_walk_event(ins walker: Walker) -> Result[Option[WalkEvent], IoError];
open verb close_walker(dat walker: Walker) -> Result[Int, IoError];
```

The walker enforces depth limits, detects symlink cycles when links are
followed, preserves requested ordering, and exposes an explicit policy for
permission failures: fail-fast or return an error event. In either policy,
the permission classification remains `IoError.PermissionDenied` at the Actus
boundary; it is never rewritten as generic `IoError.Failed`. It remains
streaming.

### Mutation and publication

The library provides explicit mutation verbs:

```act
create_dir(path)
create_dir_all(path)
remove_file(path)
remove_dir(path)
remove_dir_all(path)
rename(from, to)
copy_file(from, to)
hard_link(from, to)
symlink(from, to)
```

Recursive deletion is never implicit. Rename is atomic only within the host
filesystem; a cross-filesystem attempt returns `IoError.CrossDevice` rather
than being silently emulated as copy-and-delete. Overwrite policy and link
behavior remain target-documented and typed.

`create_dir` creates exactly one directory and never creates missing parents.
`create_dir_all` creates each missing parent in order and treats already
existing directories as success; neither operation replaces a file-shaped
component or performs recursive deletion.

For durable publication, the library provides:

```act
write_file_atomic(path, contents)
write_and_flush(path, contents)
replace_atomic(staged, active)
sync_parent_directory(path)
publish_file(staged, active)
```

The publication sequence is staged write, flush, sync, rename/replace, and
optional parent-directory sync. A failed publication must leave the previous
active file valid. Staging cleanup must not hide the primary failure.

### Permissions, links, and security

The API exposes capability-checked operations for readonly state, permissions,
ownership, symlink inspection/creation, and hard links where supported. POSIX
mode/owner behavior and Windows ACL/reparse/share behavior remain behind the
portable typed surface.

The implementation must protect against path traversal, symlink escape,
predictable temporary names, TOCTOU-sensitive check-then-open patterns, and
permission-boundary bypasses. Applications can request a sandbox root or
capability restriction. Unsupported platform features return `Unsupported`;
they are not emulated through shell commands.

### Temporary resources and locking

The library provides secure temporary resources and explicit cleanup:

```act
temp_file(options)
temp_dir(options)
temp_name()
cleanup_temp(path)
lock_shared(file)
lock_exclusive(file)
try_lock(file)
unlock(file)
```

Temporary creation uses unique names and create-new semantics. Locking and
sharing behavior are capability-checked, and stale-lock policy is explicit.
The standard library exposes shared, exclusive, try-lock, and unlock verbs.
POSIX uses `flock`; Windows uses `LockFileEx` and `UnlockFileEx`. The complete
file range is locked, non-blocking contention returns `IoError.Busy`, and
provider failures remain typed. Targets without an advisory-lock primitive
return `IoError.Unsupported`; no backend reports a false lock success.

### Errors and limits

`IoError` must distinguish, where supported:

```text
NotFound, PermissionDenied, AlreadyExists, InvalidInput, InvalidPath,
InvalidEncoding, NotDirectory, IsDirectory, AlreadyClosed, EndOfStream,
UnexpectedEof, WouldBlock, Interrupted, Busy, CrossDevice, QuotaExceeded,
TooManyOpenFiles, NameTooLong, PathTooLong, SymlinkLoop, Unsupported,
CorruptData, Failed
```

Errors should preserve operation context and native classification without
exposing native structs. Every operation is subject to a target profile with
maximum path length, component length, directory depth, entries, bytes, open
handles, and optional operation duration. Quota exhaustion is typed and never
reported as complete success.

### Async boundary and observability

Blocking filesystem operations remain explicitly synchronous. A future async
surface is separate:

```act
async_file_read(...)
async_file_write(...)
async_read_dir(...)
poll(operation)
cancel(operation)
await(operation)
```

Async operations define cancellation, partial completion, ownership transfer,
shutdown, and executor requirements. They do not make synchronous verbs
implicitly asynchronous and do not enter serialized state.

Optional caller-owned metrics may report files opened, bytes read/written,
entries visited, failures, retries, and duration. The standard library does
not print diagnostics itself and must redact path contents from default traces.

## Architectural constraints

1. Filesystem I/O is hosted and blocking unless an explicit async capability is
   selected.
2. No filesystem handle, native pointer, or host path is serializable state.
3. Directory enumeration is streaming; no public API requires an unbounded
   collection.
4. Recursive walking, symlink following, ordering, filtering, deletion, and
   publication are explicit policies.
5. The semantic compiler owns ownership and cleanup validity; code generation
   does not repair invalid handle usage.
6. Cranelift and LLVM expose equivalent public behavior and ABI layouts.
7. Freestanding targets report unavailable filesystem capabilities through
   typed results without acquiring host filesystem symbols.
8. Inference, impulse forwarding, realtime code, and other hot paths must not
   perform synchronous filesystem I/O.

## Downstream application integration

After this ADR's directory contract is implemented, a downstream application
may add:

```ini
source=directory
input_dir=./datasets/accepted/georgian-baseline
```

The application runner can discover regular input files, filter by extension,
process them in deterministic order, initialize the first generation, load the
published generation for subsequent files, and publish after each successful
file. It must preserve the last valid manifest after an intermediate failure
and emit per-file plus aggregate metrics. It must never add one configuration
entry per generated artifact or input file.

## Rejected alternatives

1. Returning `Array[Path, N]` from `read_dir`: rejected because it imposes an
   arbitrary capacity, hides allocation, and cannot represent large trees.
2. Returning a dynamic collection: rejected because ownership, allocation, and
   cleanup would be hidden from bounded Actus programs.
3. Using shell commands or `glob` subprocesses: rejected because it breaks
   portability, typed errors, ownership, security, and backend parity.
4. Making `read_dir` recursive by default: rejected because traversal cost,
   symlink policy, and failure behavior would be implicit.
5. Promising native enumeration order as deterministic: rejected because it
   varies by filesystem and platform.
6. Following symlinks by default: rejected because it permits unexpected
   traversal and cycle behavior.
7. Adding filesystem semantics to the language core: rejected because this is
   a hosted standard-library capability.
8. Implementing only Cranelift lowering: rejected because public std behavior
   must remain backend-neutral and LLVM-compatible.

## Verification

- Every public declaration has accepted and rejected parser/semantic tests.
- Directory lifecycle tests cover EOF, close, move, early return, and leaks.
- Metadata tests cover files, directories, links, missing entries, and races.
- Ordering tests prove deterministic results on both native backends.
- Walker tests cover depth, cycles, symlinks, filters, and error policies.
- Publication tests cover truncation, flush, rename, crash-recovery, and stale
  staging files.
- Security tests cover traversal, symlink escape, permissions, and temporary
  name collisions.
- Cranelift and LLVM build and execute the same filesystem fixtures.
- Hosted checks pass without filesystem symbols in freestanding output.
- Formatter, strict check, conformance, clippy, all-target tests, source-size
  checks, and documentation checks pass before the ADR becomes Accepted.
- Downstream directory ingestion is accepted only after multi-file, restart,
  malformed-input, permission, and partial-publication evidence is recorded.
