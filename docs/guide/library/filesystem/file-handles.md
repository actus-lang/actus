# File handles and streaming

## Opening

`file_open` opens an existing path for reading. `file_create` creates or
truncates a path for writing. Both borrow the `Path` only for the provider call
and return `Result[File, IoError]`.

```actus
erg opened = file_open(path: abs path);
return case dat opened {
    Result.Ok(file) => use_file(file: dat file),
    Result.Err(error) => report(error: dat error),
};
```

A failed open does not produce a usable handle and has no handle cleanup path.
A successful open creates one owned `File`.

## Reading

Reading needs an exclusive loan of both the file and destination buffer:

```actus
file_read(self: ins file, buffer: ins bytes);
```

The provider writes into existing storage and returns an exact byte count.
The library does not return a hidden newly allocated buffer from this verb.
The caller decides capacity and may reserve it before reading.

## Writing

Writing takes an `ins` loan of the file and an immutable `abs` view of the
source buffer:

```actus
file_write(self: ins file, buffer: abs bytes);
```

The source remains available after the call. A short successful count is
visible to the caller and must be handled when the application requires a
complete write.

## Flush and close

`file_flush` asks the provider to commit buffered file state. `file_close`
releases the underlying resource and marks a successful handle invalid. The
`Drop` performance invokes cleanup when the owner leaves scope, so explicit
close is useful when the application needs to observe a close result but is
not required to invent a second cleanup mechanism.

A failed close leaves the handle live for deterministic cleanup retry. Do not
copy the `File` value or extract its opaque handle for external storage.

## Cursor movement

Use `file_seek_start`, `file_seek_current`, or `file_seek_end` directly, or
use the `Seeker` performance with `SeekFrom`. Success returns the new absolute
byte position:

```actus
erg position = file_seek_current(
    self: ins file,
    offset: abs relative_offset,
);
```

The offset is signed. Seeking does not read or write payload bytes.

## Metadata from a handle

`file_metadata(ins self: File)` returns copied `Metadata` facts while retaining
ownership of the file. It does not close or transfer the handle.

## Reader, Writer, Seeker, and Drop

`File` implements the standard performances:

- `Reader.read(ins self, ins buffer)` delegates to `file_read`;
- `Writer.write(ins self, abs buffer)` delegates to `file_write`;
- `Writer.flush(ins self)` delegates to `file_flush`;
- `Seeker.seek(ins self, dat from)` dispatches on `SeekFrom`;
- `Drop.drop(ins self)` closes the resource during scope teardown.

These performances preserve the same ownership and typed-result contracts as
the named verbs.
