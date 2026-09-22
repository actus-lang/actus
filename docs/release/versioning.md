# Release and Versioning Policy

Actus is currently Alpha software. Until the first stable language edition,
the compiler, language syntax, semantic rules, and internal unit ABI may
change between minor releases or documented development milestones.

- Alpha releases use `0.y.z` versions.
- Patch releases contain compatible fixes and documentation corrections.
- Minor Alpha releases may add language features or change explicitly
  unstable behavior.
- Breaking changes require a roadmap or architecture decision update and a
  migration note when practical.
- The internal Actus unit ABI is unstable in Alpha and requires an exact
  compiler/toolchain match.
- The C ABI boundary remains the stable interoperability boundary for
  external libraries and platform runtimes.

Release notes are generated from Conventional Commits and stored under
`docs/changelog/` for tagged versions.
