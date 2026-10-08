# Runtime profiles and standard-library availability

A runtime profile selects which standard-library providers a package may use.
The profile is configured in the package manifest and checked before native
emission.

```toml
[build]
runtime = "std"
```

## Core and hosted services

The dependency-free `core` profile does not silently provide console I/O,
filesystem access, hosted strings, clocks, schedulers, or other operating
system services. A package using those capabilities must select a profile that
supplies them.

The hosted `std` profile provides the current providers for `std::io`,
`std::fs`, `std::path`, `std::string`, and `std::time`. `std::wire` and
`std::region` expose transport/provider-neutral contracts, but their target
adapters still have explicit capability boundaries.

## Profile is not hardware proof

A source import resolving successfully proves module availability in the
selected compiler profile. It does not prove that a particular freestanding
board has a console, filesystem, clock, scheduler, or transport provider.
Target integration must provide and measure the corresponding runtime
contract.

## Choosing a profile

1. Identify every standard-library capability the package calls.
2. Select the least powerful profile that supplies those providers.
3. Confirm the target has the required runtime implementation.
4. Keep hosted-only operations behind an application or adapter boundary.
5. Record profile, target, compiler version, and provider assumptions in
   reproducible build or benchmark evidence.

## Freestanding rule

A freestanding build must not silently import hosted services. If the selected
profile lacks a provider, the compiler or runtime reports the documented
capability failure before a false native implementation is emitted.

## Related guides

- [IO runtime boundary](io/runtime.md)
- [Filesystem runtime boundary](filesystem/runtime.md)
- [String runtime boundary](string/runtime.md)
- [Time runtime boundary](time/runtime.md)
- [Region runtime boundary](region/runtime.md)
- [Wire runtime boundary](wire/runtime.md)
