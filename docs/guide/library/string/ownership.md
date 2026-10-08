# Ownership and cleanup

## Borrowed inspection

These parameters are borrowed:

```actus
abs text: String
abs text: Utf8Buffer
```

The callee cannot consume the source through these APIs. `string_length`,
`string_byte_at`, `utf8_length`, and `utf8_byte_at` leave the source available
to the caller after return.

## Consuming destination storage

Constructors take the destination as `dat storage: Buffer`:

```actus
utf8_from_buffer(storage: dat storage)
utf8_from_string(text: abs source, storage: dat storage)
```

The transfer makes the lifetime decision explicit. On success, the returned
`Utf8Buffer` owns the storage. On failure, rejected storage is dropped before
the typed error is returned. The caller must not use the consumed binding as
if the call had borrowed it.

## Exclusive provider loan

During `utf8_from_string`, the provider receives the destination as an
exclusive `ins` loan while copying. This allows mutation during the provider
call without publishing a second owner. The public constructor then consumes
the storage into the result or cleans it up on failure.

## Cleanup paths

There are two valid terminal paths for constructor storage:

- success: `Buffer` becomes `Utf8Buffer.storage`;
- failure: the rejected buffer is dropped exactly once.

No partial result is returned. This is especially important for capacity,
provider, and validation failures.

## Reading does not mutate

Byte and length access uses `abs` and does not alter `storage` or `length`.
They may reject an invalid aggregate with `InvalidStorage`, but they never
repair it in place.
