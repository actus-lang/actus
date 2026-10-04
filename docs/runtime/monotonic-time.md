# Monotonic time runtime contract

The hosted runtime exposes a process-local monotonic source through a private
C ABI symbol. Actus code uses the typed `std::time` facade and never receives
the raw runtime symbol or a platform clock structure.

Provider selection is manifest-driven. A project may declare
`[build.time_provider]` in `Actus.toml`; the declaration identifies its read
symbol, unit, frequency, counter width, wrap policy, read atomicity, interrupt
safety, initialization, calibration, sleep, reset, and discontinuity rules.
The compiler validates this contract before native emission and includes it in
the target artifact identity.

The public conversion boundary is checked integer arithmetic. Bounded hardware
counters may use the explicit modulo extension policy; ambiguous backwards
samples are rejected. Unsupported freestanding providers fail closed instead
of inheriting the hosted clock.

Timing measurements are not universal performance claims. Scheduler activity,
interrupts, cache state, frequency scaling, target resolution, optimization,
and workload placement affect results. Compare fixed workloads only under a
documented, reproducible environment.
