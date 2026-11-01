# `meta fault(...)`

`meta fault("name")` requests one controlled native failure for a strict test
fixture. It exists to verify publication, cleanup, recovery, and rollback
contracts that are difficult to trigger reliably with ordinary input.

```actus
meta fault("short_write")
meta test
verb publication_preserves_the_previous_file() -> Int {
    return 0;
}
```

The fault is installed by the test runner for one child process and consumed
once at the named native filesystem phase. The normal runtime archive does not
contain the fault control path; fault-bearing tests require the compiler's
test-fault-injection configuration.

## Supported fault names

- `staging_open_failure`: staging-file creation fails before writing;
- `write_failure`: the native write fails without a partial count;
- `short_write`: the native write reports fewer bytes than requested;
- `flush_failure`: flushing the staged file fails;
- `close_failure`: closing the staged file fails after writing or flushing;
- `cleanup_failure`: pre-publication cleanup fails and leftover state is
  observable;
- `rename_failure`: publication rename fails before replacement;
- `interrupted_before_rename`: the child stops before publication rename;
- `parent_open_failure`: opening the destination parent for synchronization
  fails after publication;
- `parent_sync_failure`: destination-parent synchronization fails;
- `interrupted_publication`: the child stops after publication rename.

## Constraints

- The fault name is not free-form.
- A test may select at most one fault.
- Unknown names and duplicate fault declarations are rejected before execution.
- Fault metadata is test-only and must never become an application filesystem
  API or a production behavior switch.
- Every new fault requires a compiler/runtime implementation and a strict
  regression fixture.

