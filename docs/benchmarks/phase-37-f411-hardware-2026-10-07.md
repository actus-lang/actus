# STM32F411 Hardware Provider Evidence

## Scope

This record captures a real execution of the no-std freestanding provider core on
the connected STM32F411 through ST-LINK V3. The firmware opens one bounded
window, repeatedly writes one element, publishes the resident mirror, reads the
published descriptor, and cancels the next mutation. It reports its state from
SRAM at `0x20000000`.

This is functional target execution evidence. It is not a cycle, power, or
throughput benchmark.

## Environment

- Target profile: `STM32F411CE`
- Probe: ST-LINK V3
- Probe serial: `001D002D3235510537333439`
- Protocol: SWD
- Speed: 1000 kHz
- Core: ARM Cortex-M4 r0p1
- Flash observed: 1024 KiB
- SRAM evidence address: `0x20000000`
- Firmware package: `runtime/freestanding/f411_probe`
- Rust target: `thumbv7em-none-eabihf`

## Firmware and ELF evidence

The probe package builds without `std`, uses the provider core directly, and
links against a fixed STM32F411 flash/RAM layout. The release ELF is:

```text
ELF 32-bit LSB executable, ARM, EABI5, statically linked
text: 648 bytes
bss:   32 bytes
```

The evidence record is a fixed 32-byte `#[repr(C)]` structure at `0x20000000`.
The firmware writes `0x41494531` (`AIE1`) after opening the provider and uses
state `1` while the lifecycle loop is running. A failure path writes state
`0xEE`.

## Programming and execution commands

```text
cd runtime/freestanding/f411_probe
cargo build --release

probe-rs download \
  --chip STM32F411CE \
  --probe 0483:374f:001D002D3235510537333439 \
  --protocol swd \
  --speed 1000 \
  --verify \
  --reset \
  target/thumbv7em-none-eabihf/release/actus-f411-provider-probe
```

The download completed with verification and reset.

## SRAM evidence

First read immediately after reset:

```text
probe-rs read b32 0x20000000 8 ...
20000000: 41494531 00000001 00008188 00008194 0000819d 000081a6 000081af 0000002a
```

Second read after a one-second delay:

```text
probe-rs read b32 0x20000000 8 ...
20000000: 41494531 00000001 00607fc5 00607fcf 00607fd8 00607fe4 00607fed 0000002a
```

The fields are, in order:

```text
magic       = 0x41494531 (AIE1)
state       = 1 (running)
iterations  = 0x00607fc5
reads       = 0x00607fcf
writes      = 0x00607fd8
publishes   = 0x00607fe4
cancels     = 0x00607fed
last_value  = 42
```

The counters advance between reads and `last_value` remains `42`. The slight
counter skew is expected because the target is running while the eight words
are read over SWD; the magic, running state, advancing counters, and value are
the acceptance signals.

## Acceptance result

- [x] The provider firmware builds for `thumbv7em-none-eabihf`.
- [x] The ELF is linked for the observed F411 flash and SRAM map.
- [x] The image is programmed and verified through ST-LINK V3.
- [x] Reset-based execution reaches the provider lifecycle loop.
- [x] The provider lifecycle produces advancing SRAM counters.
- [x] The readback value is the value written through the provider (`42`).
- [ ] Cycle-level latency measurement.
- [ ] Power measurement.
- [ ] Full board application integration with target-owned allocation symbols.

## Evidence boundary

This proves that the current fixed-window provider core executes on the connected
STM32F411 and that its write, publish, read, and cancel operations progress in a
live loop. It does not prove the complete Actus generated Region bridge, a
production board startup/clock configuration, interrupt safety, multicore
synchronization, cycle budget, or power consumption.
