# Targets and build profiles

A target describes the platform, address width, ABI, linker flavor, runtime
provider, and available native services. A build profile selects compiler
optimization and output settings.

Use fixed-width Actus types for data whose representation crosses a target or
ABI boundary. Keep target-specific linker and runtime details in package
configuration and target contracts, not in general language syntax.

Hosted and freestanding profiles are separate contracts. A module available on
one target may be rejected before native emission on another target.
