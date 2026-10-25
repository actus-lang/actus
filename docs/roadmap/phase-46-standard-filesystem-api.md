# Phase 46: Production Standard Filesystem API

This roadmap defines the production filesystem surface for Actus programs.
The API is designed as a reusable standard-library capability for hosted
applications, tooling, data pipelines, and bounded systems services.

The design must remain explicit about ownership, blocking behavior, platform
differences, path semantics, and failure recovery. A directory API must not
silently become an unbounded collection, a hidden allocator, or a hot-path
filesystem dependency.

## Gate 0: Contract, scope, and capability boundary

- [ ] Accept the filesystem API as a standard-library capability, separate
      from compiler internals and application-specific ingestion logic.
- [ ] Define the hosted-filesystem target contract and the behavior of builds
      that do not provide a filesystem capability.
- [ ] Define whether filesystem verbs are blocking and document that they are
      forbidden in inference, impulse forwarding, and other realtime hot paths.
- [ ] Define the distinction between a lexical `Path`, an opened filesystem
      handle, and a canonicalized filesystem location.
- [ ] Define that filesystem paths and handles are never serialized into Actus
      application state or binary persistence formats.
- [ ] Define the platform-neutral error contract and the mapping from native
      error classes to typed `IoError` variants.
- [ ] Define the maximum path length, component length, directory depth, and
      entry-name size supported by each target profile.
- [ ] Record unsupported operations explicitly instead of silently emulating
      them with shell commands or platform-specific source code.
- [ ] Add an ADR covering ownership, cleanup, ordering, security, and target
      capability behavior before implementation expands.

## Gate 1: Canonical module and public facade

- [ ] Create the canonical `library/std/src/fs/fs.act` facade.
- [ ] Split implementation by responsibility: handles, directory iteration,
      metadata, paths, permissions, links, temporary resources, and atomic
      publication.
- [ ] Export only intentional public declarations from the facade.
- [ ] Keep native extern declarations in a platform-specific implementation
      boundary; do not leak C or OS types into the public Actus API.
- [ ] Add public documentation for every type, enum, constant, and verb.
- [ ] Add module-visibility tests proving `import std::fs;` with qualified
      `fs::...` access, and `import { open std::fs };` for intentional
      unqualified access, are the supported paths.
- [ ] Reject direct imports of private implementation children where the
      compiler's module rules require the canonical facade.

## Gate 2: Typed paths and filesystem identity

- [ ] Reuse the existing `Path` representation and document its encoding and
      platform rules.
- [ ] Add explicit path operations for `join`, `parent`, `file_name`,
      `extension`, `file_stem`, `components`, and `normalize`.
- [ ] Guarantee that every non-owning `PathComponent` view preserves a valid
      source reference and that reads through `source.storage`, `offset`, and
      `length` observe the original platform representation without exposing
      backend-specific pointer details.
- [ ] Add regression coverage for component-source byte/code-unit reads,
      including extension and file-name views on POSIX and Windows paths.
- [ ] Add a separate canonicalization operation that resolves filesystem
      identity and reports missing components instead of changing lexical
      paths implicitly.
- [ ] Define absolute, relative, root, separator, dot, and dot-dot behavior
      for POSIX and Windows targets.
- [ ] Define whether a path operation preserves or rejects embedded NUL,
      invalid UTF-8, reserved names, and platform-invalid components.
- [ ] Add `same_file` or equivalent identity comparison where the target can
      provide it, with an explicit unsupported result otherwise.
- [ ] Add accepted and rejected tests for traversal, malformed encoding,
      roots, separators, and platform-specific reserved names.

## Gate 3: Directory handle lifecycle

- [ ] Add a typed directory handle that owns one native directory resource.
- [ ] Add `read_dir(abs path: Path) -> Result[Directory, IoError]`.
- [ ] Add `next_dir_entry(ins directory: Directory) ->
      Result[Option[DirectoryEntry], IoError]`.
- [ ] Add `close_dir(dat directory: Directory) -> Result[Int, IoError]`.
- [ ] Define end-of-directory as `Ok(Option.None)`, not as a generic failure.
- [ ] Define behavior for repeated reads after EOF.
- [ ] Reject reads and close operations on an already-closed or moved handle.
- [ ] Guarantee cleanup on early return, branch exit, loop exit, and error
      paths through the Actus ownership model.
- [ ] Define whether a dropped live handle is rejected, implicitly closed, or
      diagnosed; choose one policy and test it on every backend.
- [ ] Ensure no directory handle can escape its declared ownership scope.
- [ ] Add native leak, double-close, use-after-close, and early-return tests.

## Gate 4: Directory entry and metadata model

- [ ] Add `DirectoryEntryKind` with at least `File`, `Directory`, `Symlink`,
      and `Other` variants.
- [ ] Add a documented `DirectoryEntry` containing the entry name, relative
      path component, kind, and metadata snapshot.
- [ ] Reuse the existing `Metadata` type where its fields are sufficient;
      extend it only through an intentional public API change.
- [ ] Define metadata fields for file type, byte size, readonly state,
      modification time, access time, creation time, and platform identity.
- [ ] Distinguish a missing target, an inaccessible target, and an unsupported
      metadata field.
- [ ] Define whether entry metadata follows symlinks or describes the link
      itself; expose both operations when the platform supports both.
- [ ] Add explicit `is_file`, `is_dir`, `is_symlink`, and file-type helpers.
- [ ] Add tests for empty directories, mixed entry kinds, broken symlinks,
      unreadable entries, and metadata races.

## Gate 5: Deterministic and bounded enumeration

- [ ] Keep enumeration streaming; never require an `Array` containing every
      directory entry.
- [ ] Define native enumeration order as unspecified and unsuitable for
      reproducible builds or deterministic training.
- [ ] Add an explicit ordering policy with at least `Native` and
      `Lexicographic` modes.
- [ ] Define lexicographic ordering over the canonical byte representation,
      including case sensitivity and Unicode normalization behavior.
- [ ] Decide where sorting occurs: in the standard-library iterator with
      documented bounded/resource behavior, or in a separate bounded consumer
      layer. Do not hide an unbounded allocation in the basic iterator.
- [ ] Add an optional entry filter contract for regular files, extension,
      hidden entries, and symlink policy without requiring function pointers.
- [ ] Define maximum entries, maximum name length, and overflow behavior for
      bounded profiles.
- [ ] Add tests proving identical ordering across repeated runs and both
      native backends.

## Gate 6: Recursive walking and traversal policy

- [ ] Add a separate recursive walker rather than making `read_dir` recursive.
- [ ] Define `WalkOptions` for maximum depth, follow-symlink behavior,
      directory-first/file-first order, hidden entries, and error policy.
- [ ] Add a streaming `next_walk_event` API with enter, file, symlink, leave,
      and error events where supported.
- [ ] Detect cycles when symlinks are followed and return a typed cycle error.
- [ ] Define behavior for permission failures: fail-fast, skip-with-event, or
      collect-and-return; do not silently ignore them.
- [ ] Preserve deterministic ordering at every directory level when requested.
- [ ] Add depth, cycle, symlink, permission, and very-wide-tree tests.

## Gate 7: File operations and open options

- [ ] Expose explicit `file_open`, `file_create`, `file_read`, `file_write`,
      `file_seek`, `file_flush`, and `file_close` contracts.
- [ ] Complete `OpenOptions` for read, write, append, truncate, create,
      create-new, and close-on-exec behavior where supported.
- [ ] Define partial read and partial write handling; never treat a short
      native operation as implicit success for the complete buffer.
- [ ] Add bounded file-copy and file-stream operations without loading an
      entire file into memory.
- [ ] Add file locking or an explicit unsupported result for targets that do
      not provide advisory locks.
- [x] Add durable sync operations for data and metadata where supported.
- [ ] Add tests for concurrent open modes, short I/O, EOF, locks, flush, and
      persistence failures.

Evidence (2026-10-25): `file_sync_data` and `file_sync_all` are exposed through
the hosted `std::fs` facade, Rust runtime bridges, and buffer ABI adapters.
`tests/library/fs/file.act` passes the strict synchronization fixture; the
runtime passes `cargo check --all-targets`, and the public documentation suite
passes.

## Gate 8: Mutation, publication, and recovery operations

- [ ] Add `create_dir` and an explicit `create_dir_all` for parent creation.
- [ ] Add `remove_file`, `remove_dir`, and a separately guarded recursive
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

## Gate 10: Temporary resources and workspace safety

- [ ] Add secure temporary-file and temporary-directory creation with unique
      names and create-new semantics.
- [ ] Add explicit lifetime/cleanup policy for temporary resources.
- [ ] Prevent predictable-name collisions and symlink substitution attacks.
- [ ] Add optional caller-provided workspace roots and capability restrictions.
- [ ] Ensure cleanup is attempted after failure without hiding the primary
      error.
- [ ] Add tests for collisions, abnormal exit cleanup, read-only parents, and
      concurrent creators.

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
