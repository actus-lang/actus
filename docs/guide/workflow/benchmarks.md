# Benchmarks and measurements

A benchmark records a command, compiler revision or digest, target, profile,
input size, and captured output.

A host benchmark describes host behavior. It is not a hardware measurement.
A native executable result describes linking and execution for that target; it
does not establish behavior on another target or device.

Keep reproducible benchmark instructions and results under `docs/benchmarks/`
and link them from the relevant roadmap or API page.
