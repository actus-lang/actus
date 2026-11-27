//! Every owned value is released exactly once on every exit path (ADR-0125,
//! ADR-0027, ADR-0016, ADR-0046): at the end of its scope when it is a local or
//! a `dat` parameter that nothing moved or dropped, by an explicit `drop`, and
//! on `return`, `break` and `continue`. Releasing a value runs its own `Drop`
//! performance first and then each owned field in reverse declaration order,
//! recursively; an enum releases the payload of its active variant and an
//! array its elements. Each program is built and run on every back end that is
//! built in; the output of the `Drop` roles proves what ran and in which order,
//! and valgrind, when it is installed, proves that no value leaks or is freed
//! twice (a block that stdio keeps reachable is not a leak).
#![cfg(unix)]

#[path = "struct_cleanup_native/arenas.rs"]
mod arenas;
#[path = "struct_cleanup_native/arrays.rs"]
mod arrays;
#[path = "struct_cleanup_native/element_copies.rs"]
mod element_copies;
#[path = "struct_cleanup_native/enums.rs"]
mod enums;
#[path = "struct_cleanup_native/exits.rs"]
mod exits;
#[path = "struct_cleanup_native/harness.rs"]
mod harness;
#[path = "struct_cleanup_native/large_aggregates.rs"]
mod large_aggregates;
#[path = "struct_cleanup_native/place_assignments.rs"]
mod place_assignments;
#[path = "struct_cleanup_native/structs.rs"]
mod structs;
#[path = "struct_cleanup_native/temporaries.rs"]
mod temporaries;
#[path = "struct_cleanup_native/try_positions.rs"]
mod try_positions;
