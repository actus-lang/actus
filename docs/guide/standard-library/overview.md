# Standard-library overview

The Actus standard library is organized into public facade modules. The facade
is the import boundary; implementation siblings and raw runtime bridges are
not application APIs.

The current public families are:

- `std::io` for console and stream operations;
- `std::fs` for hosted filesystem operations;
- `std::path` for owned and validated path values;
- `std::string` for bounded borrowed byte and UTF-8 operations;
- `std::time` for typed monotonic timing and timers;
- `std::region` for bounded logical-region operations;
- `std::wire` for target-neutral bounded binary frames.

Each family states its runtime and target boundary. Import only the facade and
read the API page for ownership roles, typed errors, bounds, and allocation.
