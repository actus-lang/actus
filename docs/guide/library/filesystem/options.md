# Open options

`OpenOptions` makes filesystem intent explicit. It is a value containing six
integer switches; it does not own a file handle and does not perform I/O until
`options_open` consumes it.

## Builder sequence

```actus
erg options = options_new();
erg readable = options_read(dat options);
erg writable = options_write(dat readable);
erg creatable = options_create(dat writable);
erg opened = options_open(options: dat creatable, path: abs path);
```

Every builder consumes the previous binding and returns a new complete value.
The old binding must not be reused after the transfer.

## Flags

- `options_read` enables reading.
- `options_write` enables writing.
- `options_append` places writes at the end without requesting truncation.
- `options_truncate` requests truncation of an existing file.
- `options_create` allows creation when the path does not exist.
- `options_create_new` requests exclusive creation and fails when the path
  already exists.

The library stores these switches as scalar values and passes them to the
runtime in one open request. It does not infer a missing mode from later
operations.

## Create-new behavior

Use `options_create_new` when overwriting an existing path would be unsafe:

```actus
erg base = options_new();
erg exclusive = options_create_new(dat base);
erg opened = options_open(options: dat exclusive, path: abs path);
```

The result is `Err(IoError.Failed)` when the provider rejects the exclusive
creation, including an already existing target.

## Ownership rule

`options_open` consumes the options value. On success, only the returned
`File` remains as the new resource. On failure, the scalar configuration is
finished by the call and no handle is published.

## No path retention

The path is received as `abs path`. Neither `OpenOptions` nor the returned
`File` stores a `Path`; applications that need a path later must retain their
own owned path value separately.
