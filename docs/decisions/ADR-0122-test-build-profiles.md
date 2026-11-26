# ADR-0122: Build Profiles for Test Executables

## Status

Accepted.

## Context

`actus build` and `actus run` select a build profile with `--release` or
`--profile <name>` (ADR-0114): `debug` builds at `O0`, `release` at `O3` and
`size` at `Oz`, and a project may override a profile's level in its manifest.
`actus test` has no such option. Every test executable is built with the
profile that the manifest's `[build] profile` names, `debug` when it names
none, and changing it means editing the manifest.

Since ADR-0121 a test directory builds once, so the build no longer dominates
a run. What remains is the run time of heavy tests: a test that drives a large
amount of byte-level work spends seconds at `O0` that it would not spend at
`O3`, and a suite with several such tests is slow for that reason alone. The
levers today are a longer timeout, which hides the cost instead of removing it,
and an edit of the manifest, which changes every build of the project. A
project cannot run its tests at another profile for one run, so CI cannot
check that they pass both at `O0` and with the optimizations it ships with.

## Decision

`actus test` accepts the profile options of `actus build`, with the same
meaning and the same errors:

- `--release` selects the `release` profile;
- `--profile <name>` selects `debug`, `release` or `size`; a
  `[profile.<name>]` table of the manifest overrides the optimization level of
  that profile, as for `actus build`;
- the two are exclusive, and an unknown profile is the same error as for
  `actus build`.

Without an option the profile is the manifest's `[build] profile`, `debug`
when it names none, as for `actus build`. An option overrides the manifest for
that run.

The profile applies to every test executable of the run, on both back ends.
It changes how the program is optimized, never what it means:

- Runtime checks stay in every profile. Integer overflow, out-of-range casts,
  index checks and every other check of ADR-0115 and ADR-0117 trap with the same
  report and the same location in `release` as in `debug`. A test that expects
  a trap passes or fails the same way in both.
- `meta fault` injection, the per-test process, the empty working directory,
  the timeout and the exit-status contract of ADR-0099 and ADR-0121 do not
  depend on the profile.
- A strict run (`--strict`) applies its rules in every profile.

The run header names the profile when it is not `debug`, whether an option or
the manifest selected it, so a log shows which
build ran: `running 12 tests (release) (0 filtered)`. A strict run keeps its
own mark before the profile: `running 12 tests (strict) (release) (0 filtered)`.

## Consequences

- A project can run heavy tests at the optimization level it ships with, and
  can check in CI that its tests pass both at `O0` and at `O3`.
- `actus build`, `actus run`, `actus watch` and `actus test` parse the profile
  options with one function, so a missing name, an unknown name and a second
  profile option are the same error for every command. `actus run` used to
  accept a second profile option and keep the last one; it now rejects it.
- A release test run builds slower than a debug one, because `O3` costs more
  compile time; the choice is the user's, per run.
- Different profiles give different executables, so they never share a test
  build. A run builds under one profile into directories named by its process
  and unit, and no test build is cached across runs, so nothing is shared.

## Alternatives considered

### Optimize test builds always

Rejected. `O0` builds faster, and most tests are small; the default stays the
fastest edit-test loop. It also keeps the default profile of `actus test` the
same as that of `actus build`.

### A per-test profile in metadata

Rejected. One test directory builds one executable (ADR-0121), so a profile
per test would split the directory again. A run-wide option keeps one build
per directory.
