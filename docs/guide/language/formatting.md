# Formatting Actus source

The Actus formatter normalizes spacing and indentation while preserving the
program's meaning and source documentation.

Formatting must preserve:

- triple-quoted documentation blocks;
- imports and canonical facade declarations;
- declaration order;
- comments attached to the declarations they describe;
- selectors, field access, and ownership roles;
- all tokens needed for reparsing.

Run the formatter on a file when editing its layout, then use the check form in
project validation:

```sh
actus fmt path/to/file.act
actus fmt --check path/to/file.act
```

Formatting is idempotent. If a second formatting pass changes the file, or if
documentation or imports move, treat the result as a formatter issue and add a
focused regression case.
