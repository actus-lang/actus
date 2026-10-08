# Runtime profiles

A runtime profile selects the standard-library modules and services available to
a package. The profile is configured in the package manifest and is checked
before source emission.

Hosted modules such as IO, filesystem, paths, strings, and time require a
hosted runtime contract. `std::region` and `std::wire` also define
freestanding-capable surfaces with explicit target-provider boundaries.

A freestanding build cannot silently import hosted runtime services. Select the
profile and target together, then use only the public modules allowed by that
contract. See [hosted and freestanding builds](../compiler/hosted-and-freestanding.md).
