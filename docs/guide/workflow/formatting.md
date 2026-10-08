# Formatting workflow

Format changed Actus files, then check the canonical result:

```sh
actus fmt path/to/file.act
actus fmt --check path/to/file.act
```

For compiler and repository Rust code:

```sh
cargo fmt --all -- --check
```

Review the diff after formatting. Documentation blocks, imports, facade
exports, declaration order, and ownership roles must remain attached to the
same declarations.
