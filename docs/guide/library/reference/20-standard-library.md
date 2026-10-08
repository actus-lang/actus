# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 20. Standard library

The standard library is selected through the package build/runtime setting.
New projects default to the dependency-free `core` runtime. A project that
needs hosted standard-library modules opts into `std`:

```toml
[build]
runtime = "std"
```

The compiler resolves canonical `std::io`, `std::fs`, `std::path`, and
`std::string` imports
from the packaged library. Applications must use public typed facade APIs,
not the internal C bridge symbols.

For a package that requires an integer-only generated native boundary, enable
the compiler-owned Cranelift IR audit in `Actus.toml`:

```toml
[build]
verify_no_float_ir = true
```

The setting applies to every Actus-generated native function emitted for the
package and rejects floating-point IR before object emission. It proves only
the generated Actus IR boundary. External ABI objects and separately linked
runtime objects need their own audit and are not silently covered by this
setting.

### 20.1 `std::io`

The public I/O facade includes:

- `IoError` typed error enum;
- `Reader` and `Writer` role contracts;
- `read(ins buffer: Buffer) -> Result[Int, IoError]`;
- `write(abs buffer: Buffer) -> Result[Int, IoError]`;
- `read_line(ins buffer: Buffer) -> Result[Int, IoError]`;
- `read_byte() -> Result[Int, IoError]`;
- `print(abs text: String) -> Result[Int, IoError]`;
- `println(abs text: String) -> Result[Int, IoError]`;
- `print_int(erg value: Int) -> Result[Int, IoError]`;
- `printb(abs text: Buffer) -> Result[Int, IoError]`;
- `printlnb(abs text: Buffer) -> Result[Int, IoError]`;
- `eprint_int`, `eprint`, and `eprintln` for stderr;
- `flush()` for stdout;
- `Cursor` and cursor read/write/seek/flush operations;
- buffered reader and writer types and operations;
- `copy` for reader-to-writer transfer.

Use `print` for `String`, `printb` for binary `Buffer`, and typed result
handling for failures. Do not pass a `String` to `printb` or a `Buffer` to
`print` without an explicit, documented conversion boundary.

`Result.Ok(count)` is the number of bytes processed, not merely a boolean
success flag. EOF is a typed result condition where the API defines it.

### 20.3 `std::string`

`std::string` is hosted-only until a freestanding target supplies the same
checked provider contract. It contains `StringError` for null, invalid UTF-8,
bounds, invalid-storage, and provider failures; borrowed byte inspection for
the existing `String` ABI; and `Utf8Buffer`, an owned length-delimited UTF-8
value backed by a caller-supplied `Buffer`.

Use `utf8_from_buffer(dat storage: Buffer)` for validation and ownership
transfer, or `utf8_from_string(abs text: String, dat storage: Buffer)` for a
caller-buffer-backed String-to-owned-UTF-8 conversion. Use
`utf8_length(abs text: Utf8Buffer)` for the exact byte length, and
`utf8_byte_at(abs text: Utf8Buffer, erg index: Int)` for checked byte access.
Use `std::io::print`/`println` for `String` and `printb`/`printlnb` for raw
`Buffer` bytes; these representations must not be confused.

### 20.2 `std::time`

Use `std::time` for elapsed-time measurement and explicit monotonic deadlines:

```act
import std::time;

verb measure() -> u64 {
    erg started: Instant = now();
    erg finished: Instant = now();
    erg elapsed = duration_since(later: abs finished, earlier: abs started);
    return case dat elapsed {
        Result.Ok(value) => duration_as_nanos(abs value),
        Result.Err(_) => 0u64,
    };
}
```

`Instant` is not a wall-clock timestamp. Use `Duration` for checked integer
units and arithmetic, `Deadline` for expiration, `delay` for explicit
busy-waiting, `sleep` for provider-backed scheduler cooperation, and `Timer`
for caller-owned one-shot or periodic state machines. Handle every
`Result[_, TimeError]`; do not use a raw runtime symbol or assume nanosecond
hardware precision. A fixed-workload benchmark must document target, runtime,
optimization, workload, and scheduling conditions. The general executable
example is `examples/monotonic_time/`.

### 20.3 `std::fs`

The filesystem facade includes:

- `File` owned file handle;
- `OpenOptions` for read, write, append, truncate, create, and create-new;
- `SeekFrom` and `Seeker`;
- `Metadata` and `MetadataProvider`;
- `file_open`, `file_create`, `file_read`, `file_write`, `file_flush`,
  `file_close`, and file seek operations;
- `read_to_bytes` and `read_to_string`;
- `write_file` and `write_file_atomic`;
- `remove_file`, `rename`, `copy_file`, `create_dir`, and `remove_dir`;
- metadata access through paths and file handles.

All public operations return typed `Result` contracts. Files and buffers are
owned resources and must be passed with the correct role. A failed operation
must preserve the documented owner and cleanup behavior.
`write_file_atomic` consumes a caller-selected staging `Path`, flushes the
staging file, renames it to the final destination, and removes staging on
failure. Manifest publication and directory durability remain caller policy.

### 20.4 `std::path`

The path facade includes:

- owned `Path` and `PathPlatform` representations;
- `PathError` typed failures;
- POSIX and Windows root classification;
- validated path construction from POSIX bytes, Windows UTF-16 units, or
  supported ASCII storage;
- `normalize`;
- `join`, `push`, `reserve_path`, `set_file_name`, and `set_extension`;
- component views: `parent`, `file_name`, `file_stem`, `extension`,
  `components`, and `next_component`;
- predicates: `is_absolute`, `is_relative`, `has_root`, `starts_with`, and
  `ends_with`.

Paths are not strings by accident. Preserve their platform representation and
use typed constructors. Do not manually concatenate host path separators in
protocol or filesystem code.
