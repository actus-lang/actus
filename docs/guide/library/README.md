# Actus standard library

This section is the practical guide to the public modules under
`library/std/src/`. Each library has its own directory with an index and API
reference. Module-specific ownership, failure, lifecycle, and runtime details
are kept beside the module they describe.

## Libraries

- [`std::io`](io/README.md): console streams, reader and writer contracts,
  cursors, buffering, and stream copying.
- [`std::fs`](filesystem/README.md): hosted files, metadata, options, seeking,
  and atomic file operations.
- [`std::path`](path/README.md): platform-aware path values and builders.
- [`std::string`](string/README.md): borrowed strings and owned UTF-8 buffers.
- [`std::time`](time/README.md): monotonic time, durations, deadlines, and
  timers.
- [`std::region`](region/README.md): bounded logical regions and lifecycle.
- [`std::wire`](wire/README.md): bounded binary framing and reassembly.
- [Errors and results](errors/README.md): typed failure handling used by the
  standard library.

## Package and runtime selection

The standard library is selected through the package build setting:

```toml
[build]
runtime = "std"
```

Read [runtime profiles](runtime-profiles.md) before importing hosted modules.
The compiler resolves canonical facades such as `std::io`, `std::fs`,
`std::path`, and `std::string`; application code must not import their raw
runtime bridges.

## Complete references

- [Declaration inventory for the original standard-library guide](reference/20-standard-library.md)
- [Standard-library source](../../../library/std/src/)
- [Standard-library tests](../../../tests/)

Each module guide explains its public operations beside their actual use. The
implementation source docstrings and tests remain the declaration-level and
behavioral contracts.

## How to use a library

1. Import the public facade, for example `import std::io;`.
2. Read the module's ownership page before passing buffers or resources.
3. Handle every fallible result with `case` or `?`.
4. Use the public facade; raw runtime bridges are implementation details.
5. Check the selected runtime profile before using hosted services.
