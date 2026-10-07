# STM32F411 Hardware Probe Evidence

## Scope

This record captures read-only SWD identification of the connected STM32F411
class target. No firmware was erased, flashed, or executed by this probe.

## Environment

- Probe: ST-LINK V3
- Probe serial: `001D002D3235510537333439`
- Protocol: SWD
- Speed: 1000 kHz
- Candidate profile: `STM32F411CE`

## Commands and results

```text
probe-rs list
  STLink V3 -- 0483:374f:001D002D3235510537333439

probe-rs info --chip STM32F411CE --protocol swd --speed 1000 --verbose
  ARM Chip with debug port Default
  DPv1 / Cortex-M4 ETM CoreSight components detected

probe-rs read b32 0xE000ED00 1 --chip STM32F411CE --protocol swd --speed 1000
  e000ed00: 410fc241

probe-rs read b32 0xE0042000 1 --chip STM32F411CE --protocol swd --speed 1000
  e0042000: 100f6413

probe-rs read b16 0x1FFF7A22 1 --chip STM32F411CE --protocol swd --speed 1000
  1fff7a22: 0400
```

## Interpretation

- `0x410FC241` identifies an ARM Cortex-M4 r0p1 core.
- `0x100F6413` is the observed STM32 debug identification value.
- `0x0400` is the observed 1024 KiB flash-size value.
- SWD memory reads are working through the connected ST-LINK V3.

## Boundary

This is probe and memory-read evidence only. The repository does not yet
contain an F411 startup image, linker script, board clock configuration, or
transport output path for the provider demo. No flash, reset-based execution,
cycle timing, RAM measurement, or power measurement is claimed here.
