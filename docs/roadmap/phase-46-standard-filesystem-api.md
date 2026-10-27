# Phase 46: Production Standard Filesystem API

This roadmap defines the production filesystem surface for Actus programs.
The API is designed as a reusable standard-library capability for hosted
applications, tooling, data pipelines, and bounded systems services.

The design must remain explicit about ownership, blocking behavior, platform
differences, path semantics, and failure recovery. A directory API must not
silently become an unbounded collection, a hidden allocator, or a hot-path
filesystem dependency.

## Gate 0: Contract, scope, and capability boundary

- [x] Accept the filesystem API as a standard-library capability, separate
      from compiler internals and application-specific ingestion logic.
- [x] Define the hosted-filesystem target contract and the behavior of builds
      that do not provide a filesystem capability.
- [x] Define whether filesystem verbs are blocking and document that they are
      forbidden in inference, impulse forwarding, and other realtime hot paths.
- [x] Define the distinction between a lexical `Path`, an opened filesystem
      handle, and a canonicalized filesystem location.
- [x] Define that filesystem paths and handles are never serialized into Actus
      application state or binary persistence formats.
- [x] Define the platform-neutral error contract and the mapping from native
      error classes to typed `IoError` variants.
- [x] Define the maximum path length, component length, directory depth, and
      entry-name size supported by each target profile.
- [x] Record unsupported operations explicitly instead of silently emulating
      them with shell commands or platform-specific source code.
- [x] Add an ADR covering ownership, cleanup, ordering, security, and target
      capability behavior before implementation expands.

Gate 0 evidence (2026-10-25): ADR-0086 is accepted and defines the hosted
filesystem capability boundary, blocking behavior, path and handle semantics,
serialization restrictions, typed errors, target limits, unsupported
operations, ownership, cleanup, ordering, security, and target profiles.

## Gate 1: Canonical module and public facade

- [x] Create the canonical `library/std/src/fs/fs.act` facade.
- [x] Split implementation by responsibility: handles, directory iteration,
      metadata, paths, permissions, links, temporary resources, and atomic
      publication.
- [x] Export only intentional public declarations from the facade.
- [ ] Keep native extern declarations in a platform-specific implementation
      boundary; do not leak C or OS types into the public Actus API.
- [x] Add public documentation for every type, enum, constant, and verb.
- [x] Add module-visibility tests proving `import std::fs;` with qualified
      `fs::...` access, and `import { open std::fs };` for intentional
      unqualified access, are the supported paths.
- [x] Reject direct imports of private implementation children where the
      compiler's module rules require the canonical facade.

Gate 1 evidence (2026-10-25): `library/std/src/fs/fs.act` is the canonical
facade and `tests/library/fs/fs.act` passes strict qualified facade coverage
for directory lifecycle and metadata access.

Gate 1 implementation evidence (2026-10-25): permissions, atomic publication,
symbolic-link inspection, and temporary resources are now isolated behind
responsibility-specific children (`permissions.act`, `publication.act`,
`links.act`, and `temporary.act`). The remaining facade/export audit is still
open, so the split checkbox remains intentionally unchecked.

Gate 1 visibility evidence (2026-10-25): `tests/library/fs/fs.act` now covers
both qualified `fs::...` access and intentional unqualified access through
`import { open std::fs };`; both forms pass strict execution.

Gate 1 facade evidence (2026-10-26): responsibility-specific children are
exported only through `library/std/src/fs/fs.act`; public declaration
documentation is enforced by the documentation suite, and
`tests/stdlib_visibility.rs` rejects direct resolution of `std::fs::file`.
Native runtime symbols remain behind `src/runtime/fs`; the Actus FFI boundary
is intentionally retained as a separate follow-up for the platform ABI gate.

### Gate 1.1: Secure temporary resources

- [x] Add secure temporary-file and temporary-directory creation below a
      caller-selected workspace root.
- [x] Use OS-provided entropy, atomic create-new/mkdir semantics, bounded
      collision retries, and rejection of symlink workspace roots.
- [x] Return the generated path with the owned resource and provide explicit
      cleanup plus best-effort Drop cleanup.
- [x] Add mirrored strict fixtures for creation, cleanup, and prefix rejection.

Gate 1.1 evidence (2026-10-25): the runtime exposes secure temporary file and
directory creation with caller-selected roots, OS entropy (`/dev/urandom` on
Unix and `BCryptGenRandom` on Windows), atomic creation, bounded retries, and
typed Actus cleanup wrappers. `tests/library/fs/temporary.act` covers both
resource kinds, cleanup removal, and unsafe-prefix rejection. Adversarial
collision/concurrency and read-only-parent matrices remain tracked in Gate 10.

## Gate 2: Typed paths and filesystem identity

- [x] Reuse the existing `Path` representation and document its encoding and
      platform rules.
- [x] Add explicit path operations for `join`, `parent`, `file_name`,
      `extension`, `file_stem`, `components`, and `normalize`.
- [x] Guarantee that every non-owning `PathComponent` view preserves a valid
      source reference and that reads through `source.storage`, `offset`, and
      `length` observe the original platform representation without exposing
      backend-specific pointer details.
- [x] Add regression coverage for component-source byte/code-unit reads,
      including extension and file-name views on POSIX and Windows paths.
- [x] Add a separate canonicalization operation that resolves filesystem
      identity and reports missing components instead of changing lexical
      paths implicitly.
- [x] Define absolute, relative, root, separator, dot, and dot-dot behavior
      for POSIX and Windows targets.
- [x] Define whether a path operation preserves or rejects embedded NUL,
      invalid UTF-8, reserved names, and platform-invalid components.
- [x] Add `same_file` or equivalent identity comparison where the target can
      provide it, with an explicit unsupported result otherwise.
- [x] Add accepted and rejected tests for traversal, malformed encoding,
      roots, separators, and platform-specific reserved names.

Gate 2 lexical-path evidence (2026-10-26): `std::path::Path` owns validated
raw POSIX bytes or target-endian Windows UTF-16 units, while component views
retain their source path and offsets. The mirrored fixtures
`tests/library/path_component_source.act`, `path_components.act`,
`path_platform.act`, `path_edge_cases.act`, `path_iteration.act`, and
`path_builders.act` cover source reads, POSIX/Windows roots and separators,
lexical dot and dot-dot normalization, component extraction, ownership, and
embedded-null/invalid-encoding failures. Filesystem identity resolution,
reserved-name policy, traversal semantics, and `same_file` remain covered;
consumer-specific symlink escape policy remains open for Gate 6 and Gate 9.

Gate 2 canonicalization evidence (2026-10-26): `std::fs::canonicalize` uses
the host filesystem resolver, returns a new owned absolute `Path`, preserves
the lexical input, and rejects missing components. The mirrored
`tests/library/fs/operations.act` fixture covers both an existing path and a
missing path under strict execution.

Gate 2 path-input policy evidence (2026-10-26): constructors reject embedded
NUL units, malformed encoding, missing terminators, and inconsistent storage.
Valid lexical names that a host may reserve are preserved by `std::path`; the
filesystem operation owns enforcement and returns typed failure. The Windows
fixture `windows_reserved_name_remains_lexical`, together with the existing
embedded-NUL and invalid-UTF-16 fixtures, records this boundary.

Gate 2 identity evidence (2026-10-26): `std::fs::same_file` compares POSIX
device/inode or Windows volume/file-index identity, follows host metadata
semantics, and returns `IoError.Unsupported` where no stable identity is
available. The mirrored filesystem operations fixture covers equal existing
paths.

## Gate 3: Directory handle lifecycle

- [x] Add a typed directory handle that owns one native directory resource.
- [x] Add `read_dir(abs path: Path) -> Result[Directory, IoError]`.
- [x] Add `next_dir_entry(ins directory: Directory) ->
      Result[Option[DirectoryEntry], IoError]`.
- [x] Add `close_dir(ins directory: Directory) -> Result[Int, IoError]`.
- [x] Define end-of-directory as `Ok(Option.None)`, not as a generic failure.
- [x] Define behavior for repeated reads after EOF.
- [x] Reject reads and close operations on an already-closed or moved handle.
- [x] Guarantee cleanup on early return, branch exit, loop exit, and error
      paths through the Actus ownership model.
- [x] Define whether a dropped live handle is rejected, implicitly closed, or
      diagnosed; choose one policy and test it on every backend.
- [x] Ensure no directory handle can escape its declared ownership scope.
- [x] Add native leak, double-close, use-after-close, and early-return tests.

Gate 3 initial lifecycle evidence (2026-10-27): `Directory` owns one opaque
host iterator, `read_dir` opens it, `next_dir_entry` streams bounded owned
entry paths, and `close_dir(ins directory)` invalidates the owner after
release. The mirrored `tests/library/fs_directory.act` fixture covers valid
opening, missing-directory rejection, entry closure, repeated EOF reads, and
early-return Drop cleanup; closed-handle reuse is now covered. Moved-handle
rejection is covered by the compiler-level
`std_directory_moved_owner_is_rejected_before_codegen` regression. The native
runtime test verifies that the directory token table returns to its prior size
after close, rejects a second close, and rejects reads after close; the strict
Actus fixture covers early return, branch exit, loop exit, and typed-error
cleanup with seven strict tests passing. A Directory value may leave a scope
only through an explicit owned transfer or return; borrowed and moved owners
cannot be reused or escape through the semantic boundary.

## Gate 3.1: Branch-aware native cleanup lowering

This sub-gate is an Actus compiler prerequisite for Gate 3 cleanup and Drop
claims. Cleanup operands must remain tied to the owner value that is live in
the block where cleanup is emitted.

- [x] Represent branch-local owner bindings with their block-local SSA values;
      do not resolve cleanup operands from a single mutable name-to-value map.
- [x] Lower cleanup independently for `case` branches, loop bodies, loop
      exits, `return`, `break`, `continue`, and error paths.
- [x] Validate before backend emission that every cleanup operand is defined in
      the block or is supplied by a dominating predecessor/block parameter.
- [x] Preserve exactly-once cleanup for `Buffer`, `Region`, aggregate values,
      and user-defined `Drop` performances.
- [x] Verify equivalent behavior in every supported native backend, including
      Cranelift's verifier and linker path.
- [x] Add compiler regressions for nested `case` plus loop cleanup, early
      return, branch exit, repeated cleanup prevention, and moved owners.
- [x] Add the mirrored standard-library regression proving `Directory`'s
      implicit-close policy without changing its public `ins` contract.
- [x] Record the compiler revision, backend matrix, strict fixture output, and
      the absence of non-dominating-value verifier failures.

Gate 3.1 implementation evidence (2026-10-26): cleanup lowering now snapshots
active bindings at block entry and uses the matching snapshot for distributed
scope cleanup and case-scope exit. This prevents the previous Cranelift
non-dominating-value failure where one cleanup reused SSA value `29` in blocks
whose live owners were `17` and `27`. The compiler regression and mirrored
directory fixture pass. The backend contract now computes block dominators and
rejects cleanup operands that are undefined, sibling-branch values, or defined
after the cleanup in the same block; a direct regression covers the rejected
sibling-branch case. Exactly-once cleanup is covered for Buffer, Region,
partial aggregate fields, and user-defined Drop performances across the shared
lowering regressions. The backend parity suite now runs 11 hosted/freestanding
Cranelift/LLVM fixtures, including the `std::fs::read_dir` early-return Drop
fixture; all 11 pass. Native leak and double-close coverage pass on the
runtime bridge test. Final Gate 3.1 evidence at compiler revision `d5803f0a`:
the Rust lib suite is 150/150, the Cranelift/LLVM backend matrix is 11/11,
the strict directory fixture is 5/5, documentation validation is 4/4, and no
non-dominating-value verifier or linker failure was observed.

## Gate 4: Directory entry and metadata model

- [x] Add `DirectoryEntryKind` with at least `File`, `Directory`, `Symlink`,
      and `Other` variants.
- [x] Add a documented `DirectoryEntry` containing the entry name, relative
      path component, kind, and metadata snapshot.
- [x] Reuse the existing `Metadata` type where its fields are sufficient;
      extend it only through an intentional public API change.
- [x] Define metadata fields for file type, byte size, readonly state,
      modification time, access time, creation time, and platform identity.
- [x] Distinguish a missing target, an inaccessible target, and an unsupported
      metadata field.
- [x] Define whether entry metadata follows symlinks or describes the link
      itself; expose both operations when the platform supports both.
- [x] Add explicit `is_file`, `is_dir`, `is_symlink`, and file-type helpers.
- [x] Add tests for empty directories, mixed entry kinds, and broken symlinks.
- [ ] Add portable tests for unreadable entries and metadata races.

Gate 4 entry evidence (2026-10-27): `std::fs` now exposes the documented
`DirectoryEntryKind` and owned `DirectoryEntry` values through its canonical
facade. `next_dir_entry` fills both owned path components and a copied
metadata snapshot through the native runtime bridge. Classification uses
`symlink_metadata`, so `Symlink` describes the link itself and does not depend
on its target being present. The read-only classification helpers are exposed
through the same facade and do not perform a second filesystem lookup. The
native classification regression covers regular files, empty directories,
mixed file/directory entries with EOF, and broken symlinks; the strict mirrored
directory fixture also passes. Portable unreadable-entry and metadata-race
coverage remains open because those conditions require a stable cross-platform
test harness rather than host-specific permission or timing assumptions.
Metadata now carries signed Unix-second timestamps with explicit available,
unsupported, and unavailable statuses, plus an opaque platform identity pair;
native and path-level strict coverage validate the available POSIX snapshot.
The detailed metadata verbs preserve not-found, permission-denied,
unsupported, and generic provider failures while compatibility verbs retain
the existing `IoError` surface. Mixed-kind/error-matrix coverage remains open.

## Gate 5: Deterministic and bounded enumeration

- [x] Keep enumeration streaming; never require an `Array` containing every
      directory entry.
- [x] Define native enumeration order as unspecified and unsuitable for
      reproducible builds or deterministic consumers.
- [x] Add an explicit ordering policy with at least `Native` and
      `Lexicographic` modes.
- [x] Define lexicographic ordering over the canonical byte representation:
      comparison is case-sensitive and performs no Unicode normalization.
- [x] Perform sorting in the standard-library iterator through a bounded
      snapshot; do not hide an unbounded allocation in the basic iterator.
- [x] Add an optional entry filter contract for regular files, extension,
      hidden entries, and symlink policy without requiring function pointers.
- [x] Define maximum entries, maximum name length, and overflow behavior for
      bounded profiles.
- [x] Add tests proving identical ordering across repeated runs and both
      native backends.

Gate 5 initial evidence (2026-10-27): the basic iterator remains bounded to
one native directory handle and one returned entry at a time; it does not
materialize an array of entries. Native order is explicitly unspecified in
the filesystem guide and is not a reproducible ordering contract. The ordered
iterator provides bounded lexicographic sorting with a 4096-entry limit and
the native regression confirms bytewise `alpha` before `zeta`. Declarative
filtering now covers regular files, directories, symlinks, leading-dot names,
and exact extensions without function pointers. The backend-parity fixture
`hosted_cli_backends_preserve_bounded_lexicographic_directory_order` creates
the same `alpha`/`zeta` entries, runs the ordered iterator twice under both
Cranelift and LLVM, and verifies the same result each time. The parity run also
closed a shared-lowering defect: an imported performance cleanup symbol could
be declared as external in the module that defined it, producing a duplicate
LLVM symbol and a link failure. Module assembly now removes external entries
whose symbol is defined by the same lowered module.

## Gate 6: Recursive walking and traversal policy

- [x] Add a separate recursive walker rather than making `read_dir` recursive.
- [x] Define `WalkOptions` for maximum depth, follow-symlink behavior,
      directory-first/file-first order, hidden entries, and error policy.
- [x] Add a streaming `next_walk_event` API with enter, file, symlink, leave,
      and error events where supported.
- [x] Detect cycles when symlinks are followed and return a typed cycle error.
- [x] Define behavior for permission failures: fail-fast, skip-with-event, or
      collect-and-return; do not silently ignore them.
- [x] Preserve deterministic ordering at every directory level when requested.
- [x] Add depth, cycle, symlink, permission, and very-wide-tree tests.

Gate 6 implementation evidence (2026-10-27): the separate bounded
walker API is implemented behind `std::fs::fs.act`, with depth-first streaming
events, explicit native/lexicographic ordering, hidden-entry filtering,
non-followed symlink events, followed-link cycle detection, and fail-fast or
error-event policies. Native runtime coverage is 8/8 for depth, cycle,
broken-link, symlink, very-wide-tree overflow, permission-event mapping, and
permission fail-fast status separation; the mirrored strict fixture is 3/3.
The public `IoError.PermissionDenied` variant preserves native permission
classification instead of collapsing it into `IoError.Failed`. Fail-fast
permission status uses a distinct ABI value and cannot be confused with clean
EOF. OS-specific permission matrices (POSIX mode bits and Windows ACLs) remain
a platform follow-up; this gate's portable contract is covered by deterministic
runtime mapping tests.

## Gate 7: File operations and open options

- [x] Expose explicit `file_open`, `file_create`, `file_read`, `file_write`,
      `file_seek`, `file_flush`, and `file_close` contracts.
- [x] Complete `OpenOptions` for read, write, append, truncate, create,
      create-new, and close-on-exec behavior where supported.
- [x] Define partial read and partial write handling; never treat a short
      native operation as implicit success for the complete buffer.
- [x] Add bounded file-copy and file-stream operations without loading an
      entire file into memory.
- [x] Add file locking or an explicit unsupported result for targets that do
      not provide advisory locks.
- [x] Add durable sync operations for data and metadata where supported.
- [x] Add tests for concurrent open modes, short I/O, EOF, locks, flush, and
      persistence failures.

Evidence (2026-10-27): `File` exposes open/create/read/write/seek/flush/close,
with explicit short-I/O and EOF contracts. `OpenOptions` covers all seven
scalar switches; close-on-exec is applied atomically with POSIX `O_CLOEXEC`,
uses Windows' non-inheritable standard handle behavior, and rejects unsupported
targets. `copy_file` and the fixed-buffer `io::copy_stream` avoid whole-file
loading. `file_sync_data` and `file_sync_all` are exposed through the hosted
facade, runtime bridges, and buffer ABI adapters. Strict file/operations
fixtures and platform-selected Unix/Windows option fixtures cover the public
contract. POSIX and Windows lock backends now cover shared, exclusive,
non-blocking, and unlock operations; contention is `IoError.Busy` and targets
without a backend return `IoError.Unsupported`. Mirrored strict fixtures cover
two-handle contention, write/flush/reopen/read, EOF, cleanup, and failed
publication. The Gate 7 file-operation validation matrix is complete.

## Gate 8: Mutation, publication, and recovery operations

- [x] Add `create_dir` and an explicit `create_dir_all` for parent creation.
- [x] Add `remove_file`, `remove_dir`, and a separately guarded recursive
      removal operation; recursive deletion must never be implicit.
- [ ] Add `rename` with same-filesystem and cross-filesystem error semantics.
- [ ] Add `copy_file` with explicit overwrite and metadata policies.
- [ ] Add atomic write helpers using staged file, flush, rename, and optional
      parent-directory sync ordering.
- [ ] Add atomic replace and compare-before-publish operations where the
      target backend can provide them.
- [ ] Define crash-recovery guarantees and the exact point at which a write is
      considered published.
- [ ] Add failure-injection tests for truncated writes, failed flushes,
      failed renames, stale staging files, and interrupted publication.

## Gate 9: Links, permissions, and platform capabilities

- [ ] Add read-only symlink inspection and explicit symlink creation where
      supported.
- [ ] Add hard-link support only behind a capability-checked API.
- [ ] Add permission/read-only operations with a platform-neutral typed model.
- [ ] Preserve unknown permission bits instead of silently rewriting them.
- [ ] Define Windows read-only, ACL, reparse-point, and sharing-mode behavior.
- [ ] Define POSIX mode, ownership, umask, mount, and inode behavior without
      exposing platform types in portable source.
- [ ] Expose capability discovery so applications can choose supported behavior
      without probing by destructive failure.
- [ ] Add security tests for symlink escape, path traversal, permission
      boundaries, and race-prone check-then-open sequences.

Gate 9 implementation evidence (2026-10-25): the standard filesystem exposes
read-only `symlink_metadata` and `is_symlink` operations without following the
link target, with mirrored strict coverage for a regular directory. Symlink
creation, capability discovery, and the broader security matrix remain open.

## Gate 10: Temporary resources and workspace safety

- [x] Add secure temporary-file and temporary-directory creation with unique
      names and create-new semantics.
- [x] Add explicit lifetime/cleanup policy for temporary resources.
- [x] Prevent predictable-name collisions and symlink substitution attacks.
- [x] Add optional caller-provided workspace roots and capability restrictions.
- [x] Ensure cleanup is attempted after failure without hiding the primary
      error.
- [x] Add tests for collisions, abnormal exit cleanup, read-only parents, and
      concurrent creators.

Gate 10 evidence (2026-10-26): runtime tests cover bounded collision retry,
preservation of an occupied file, eight concurrent directory creators,
non-directory and read-only workspace rejection, and symlink-root rejection.
The application integration test
`temporary_file_drop_removes_resource_after_early_exit` verifies that
best-effort Drop cleanup removes an uncommitted
temporary file after an early process exit. All six runtime security tests and
the application regression pass.

## Gate 11: Async and background filesystem boundary

- [ ] Define a separate async or background API; do not make blocking verbs
      appear non-blocking by naming convention.
- [ ] Specify cancellation, ownership transfer, completion, and error delivery
      for asynchronous file and directory operations.
- [ ] Keep async handles out of serialized Actus state.
- [ ] Define executor/runtime requirements and the behavior on freestanding
      targets without an executor.
- [ ] Add tests for cancellation, partial completion, shutdown, and resource
      cleanup.
- [ ] Keep the synchronous API complete and correct independently of async
      support.

## Gate 12: Observability, quotas, and operational safety

- [ ] Add optional operation-level diagnostics without printing from the std
      library itself.
- [ ] Expose bytes read/written, entries visited, retries, and duration only
      through an explicit caller-owned metrics surface.
- [ ] Add quotas for maximum bytes, entries, depth, open handles, and operation
      time where the host profile supports them.
- [ ] Define quota exhaustion as typed failure with no partial-success claim.
- [ ] Add tracing hooks that cannot leak path contents or credentials by
      default.
- [ ] Add tests for quota boundaries, one-over-limit behavior, and redacted
      diagnostics.

## Gate 13: Backend parity and native ABI

- [ ] Implement the API through the shared standard-library boundary rather
      than duplicating public semantics in Cranelift and LLVM.
- [ ] Add equivalent native lowering for every supported filesystem intrinsic.
- [ ] Verify handle, path, metadata, integer-width, and error layouts on both
      backends.
- [ ] Add standalone Cranelift and LLVM build/run tests for directory opening,
      iteration, EOF, close, file filtering, and publication.
- [ ] Verify `PathComponent` source-view layout and field access equivalently
      on Cranelift and LLVM; reject the change if either backend returns a
      valid range with an invalid source view.
- [ ] Add negative tests proving unsupported target capabilities fail
      deterministically on both backends.
- [ ] Verify no filesystem operation introduces floating-point state or an
      unapproved allocation into generated engine code.
- [ ] Record compiler revision, target, backend, host profile, and filesystem
      conditions for native evidence.

## Gate 14: Downstream application integration contract

- [ ] Define a downstream application's directory-input configuration only
      after the std filesystem API is accepted.
- [ ] Discover regular `.twf` files, filter by extension, and order them
      deterministically.
- [ ] Initialize the first file and load the published generation for every
      subsequent file.
- [ ] Publish after each successful file and preserve the last valid manifest
      after any failure.
- [ ] Emit file-level and aggregate metrics with stable exit codes.
- [ ] Add one-file, multi-file, unordered, empty, malformed, permission,
      partial-publication, and restart tests.
- [ ] Keep downstream artifact discovery manifest-driven; never add one
      configuration entry per generated artifact.
- [ ] Record the Actus compiler commit and std-library capability profile in
      downstream run metadata.

## Gate 15: Documentation, examples, and completion

- [ ] Add a complete standard-library filesystem chapter to the Actus guide.
- [ ] Add examples for safe file reading, streaming directory iteration,
      deterministic traversal, atomic publication, and cleanup.
- [ ] Document blocking and hot-path restrictions prominently.
- [ ] Document target capability differences and unsupported operations.
- [ ] Add migration notes for applications using ad-hoc native filesystem
      bindings.
- [ ] Run formatter, strict check, conformance, clippy, all-target tests, and
      backend parity tests.
- [ ] Confirm source-size and function-size limits for every new module.
- [ ] Confirm no public facade bypass, hidden dynamic collection, or leaked OS
      handle remains.
- [ ] Update the ADR, language guide, standard-library index, and release notes.
- [ ] Mark the roadmap complete only after all gates have reproducible green
      evidence.
