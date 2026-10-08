# Ownership and lifecycle

## Roles at the API boundary

- `abs Path` borrows path data for one provider call.
- `abs Buffer` borrows immutable write contents.
- `ins File` loans an owned handle exclusively while reading, writing,
  flushing, seeking, or closing.
- `ins Buffer` loans existing destination storage for a read.
- `dat OpenOptions` transfers the option value into `options_open` or the next
  builder.
- `dat Path` transfers a staging path to `write_file_atomic` so cleanup has a
  single owner.

## File resource

A successful open creates exactly one `File` owner. The `File` stores an
opaque scalar handle and not a raw operating-system pointer or a `Path`. The
normal `Drop` performance closes the handle at scope exit.

Explicit `file_close` is useful when the application needs the close result or
needs to release the resource before the enclosing scope ends. A successful
close invalidates the handle; a failed close keeps cleanup observable.

## Buffer resource

`read_to_bytes` and `read_to_string` return an owned `Buffer`. The caller owns
that result and must preserve or drop it according to the surrounding Actus
scope. A failed read drops its temporary output before returning the error.

Read-into-handle operations use a caller-owned buffer and do not transfer it.
The `ins` loan ends when the call returns.

## Atomic staging resource

The staging path remains owned by the atomic operation until publication or
failure cleanup. After a successful rename, the operation drops the staging
binding. After any earlier failure it attempts to remove the staging file,
drops the path, and returns the original error.

## Result handling

Every filesystem result must be branched explicitly. A failed open cannot be
read or closed as a file. A failed seek must not be treated as if the cursor
moved. A short write must not be reported as a complete write without an
application decision.
