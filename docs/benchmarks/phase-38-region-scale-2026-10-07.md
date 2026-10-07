# Phase 38 Region Scale Evidence — 2026-10-07

## Scope

This record measures the current Actus compiler's `Region[T]` implementation
for a primitive `u8` element and a fully covered 64-byte packed `Minicolumn`
element. The logical lengths are 1 MiB, 1 GiB, and 1 TiB. Each case uses one
resident element, so the measurement tests logical capacity independently from
resident storage.

The packed fixture is:

```act
pack Minicolumn {
    erg storage: Array[u8, 64];
    layout little;
    fields {
        erg marker: u8 at 0;
        abs _reserved_0: u128 at 8 = 0;
        abs _reserved_1: u128 at 136 = 0;
        abs _reserved_2: u128 at 264 = 0;
        abs _reserved_3: u120 at 392 = 0;
    }
}
```

## Reproduction

From the Actus repository root:

```sh
cargo test --test std_region_scale -- --nocapture --test-threads=1
```

The peak RSS measurements were collected by launching the built
`target/debug/actus` compiler for each generated fixture and polling the
compiler process and its descendants with `psutil` at 5 ms intervals. Object
size and defined-symbol measurements used `actus build --strict --emit obj`,
`stat`, and `nm -S --defined-only`.

Compiler source revision: `59fc983` (`phase-38-production-logical-region`).
Host profile: Linux x86-64, hosted `std` runtime, strict build, zero-float IR
verification enabled. These are host measurements and do not claim embedded
hardware performance.

## Executable scale results

| Element | Logical length | Resident bytes | Peak compiler RSS | Build time | Executable bytes | Run status |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `u8` | 1 MiB | 1 | 56,336 KiB | 76 ms | 5,775,568 | 0 |
| `u8` | 1 GiB | 1 | 56,692 KiB | 79 ms | 5,775,568 | 0 |
| `u8` | 1 TiB | 1 | 56,508 KiB | 78 ms | 5,775,568 | 0 |
| `Minicolumn` | 1 MiB | 64 | 56,368 KiB | 77 ms | 5,775,584 | 0 |
| `Minicolumn` | 1 GiB | 64 | 56,456 KiB | 67 ms | 5,775,584 | 0 |
| `Minicolumn` | 1 TiB | 64 | 57,880 KiB | 65 ms | 5,775,584 | 0 |

The test output also reported successful checked reads for every row. The
resident window remained one element while logical capacity increased by six
orders of magnitude.

## Object and symbol results

Object-only builds were measured at 1 MiB and 1 TiB. The defined symbol count
was one in every fixture; the largest defined symbol was the entry function.

| Element | Logical length | Object bytes | Defined symbols | Largest defined symbol |
| --- | ---: | ---: | ---: | ---: |
| `u8` | 1 MiB | 1,576 | 1 | 136 bytes |
| `u8` | 1 TiB | 1,568 | 1 | 128 bytes |
| `Minicolumn` | 1 MiB | 1,592 | 1 | 144 bytes |
| `Minicolumn` | 1 TiB | 1,584 | 1 | 136 bytes |

## Acceptance boundary

- Logical capacity does not materialize one compiler object per logical
  element.
- Resident storage remains bounded by the configured window.
- Primitive and packed Region elements both pass strict build and execution at
  MiB, GiB, and TiB logical lengths.
- Executable and object sizes, symbol count, and peak compiler RSS remain
  effectively constant across the tested logical capacities.
- The evidence covers the supported sized element categories only. Unsupported
  incomplete or malformed aggregate types remain rejected by semantic tests.
