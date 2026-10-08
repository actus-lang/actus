# Runtime and target behavior

## Hosted boundary

The current `std::fs` implementation uses private hosted provider bridges for
opening handles, reading and writing buffers, flushing, closing, seeking,
metadata, and path operations. The public facade translates provider status
integers into `IoError` and never exposes those integers to applications.

The handle is an opaque scalar token managed by the runtime. It is not a raw
OS pointer and must not be serialized or placed in persistent state.

## No hidden allocation contract

Path arguments are borrowed views. File reads use caller-provided storage or
an explicitly returned owned buffer. Open options are scalar flags. The
facade does not secretly retain paths, duplicate handles, or grow caller
buffers during a single handle operation.

One-shot reads intentionally return owned storage because their purpose is to
collect the complete file. That ownership is visible in the result type and
is different from streaming into an `ins Buffer`.

## Target capability

Filesystem access is hosted. A freestanding target may expose `std::fs` only
when it provides equivalent handle, path, metadata, and error behavior. An
import that parses successfully does not prove that a target has a filesystem
provider.

## Synchronization and blocking

The public API does not promise non-blocking execution or a fixed latency.
Providers may wait on storage. Applications must place calls at explicit
boundaries and must not use them as part of deterministic impulse evaluation
or another real-time hot path.

## Verification boundary

The Actus tests cover facade imports, empty-path rejection, file handles,
read/write/flush/seek behavior, metadata, options, directories, text
validation, create-new behavior, and atomic error cleanup. Those tests prove
hosted contract behavior; they do not prove any particular embedded storage
controller or hardware filesystem.
