# ADR-0034: Target and Board Manager

- Status: Proposed
- Date: 2026-09-27
- Scope: Actus target profiles, board workflows, and VS Code integration

## Context

Actus supports host and freestanding ambitions, but building, flashing, and
simulating a device require target-specific tools, memory maps, probes, and
reset behavior. Hardcoding board names or shell commands in the extension
would make the workflow opaque, unsafe, and impossible to maintain across
platforms.

## Decision

Targets and boards are configuration-driven profiles consumed by the compiler
CLI and presented by the extension. The extension owns discovery, selection,
validation, progress reporting, and cancellation; it does not own target
semantics or invent compiler flags.

A profile declares:

- stable target identifier and human-readable name;
- compiler target triple/profile and feature set;
- ABI, pointer width, endianness, and supported primitive/pack capabilities;
- `std` capability set (`host_io`, `filesystem`, `allocator`, or none);
- linker script, startup object, and artifact kinds;
- flash/debug/simulation providers and required executable capabilities;
- probe identity filters and reset policy;
- timeouts, log locations, and artifact retention policy; and
- whether the profile is host, simulator, or physical hardware.

Profiles are validated before use and are never silently merged with host
defaults. The compiler remains responsible for code generation and artifact
correctness.

## Workflows

The manager provides explicit actions:

1. select and validate a profile;
2. configure and show the exact build invocation;
3. build into a deterministic artifact directory;
4. optionally flash after confirmation;
5. optionally launch a simulator or debugger; and
6. stream structured logs with cancellation and exit status.

Flash and erase operations require a visible confirmation and must identify the
selected board, probe, artifact, and destructive scope. A dry-run is always
available. The extension must not execute arbitrary values from source files.

## Provider boundary

Flash, debug, and simulation backends implement a versioned provider contract.
Providers report capabilities, validation errors, progress, cancellation, and
structured exit results. The manager never assumes that `probe-rs`, OpenOCD,
J-Link, QEMU, or Renode is installed; availability is discovered and shown to
the user. Provider output is treated as untrusted text and is escaped in the
UI.

## Verification

- [ ] Profile schema and target-capability negotiation are versioned.
- [ ] Tests cover unsupported host services on freestanding targets.
- [ ] Dry-run tests verify exact commands without invoking devices.
- [ ] Integration tests cover build, cancellation, timeout, and provider
      failure states.
- [ ] Hardware tests are separate from compiler CI and record board/probe
      identity, artifact hash, and reset outcome.
- [ ] No profile contains a project-specific operating-system dependency.

## Non-goals

This ADR does not choose a particular board, vendor, flashing tool, simulator,
or operating-system distribution. It does not promise hardware success from a
host-only test and does not change the Actus target model.
