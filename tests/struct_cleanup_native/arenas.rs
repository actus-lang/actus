//! An arena is released at every exit of its scope in one bulk step
//! (ADR-0030, ADR-0125, bug #94): its storage is a fixed slot of the verb,
//! emptied when the arena is built and when it is released, so an arena
//! declared in a loop reuses the same storage on every iteration on both back
//! ends instead of growing the stack.

use super::harness::{BACKENDS_BUILT, PRELUDE, run_on_every_backend};

#[test]
fn an_arena_declared_in_a_loop_reuses_its_storage_on_every_iteration() {
    let source = format!(
        "{PRELUDE}struct Node {{ value: Int, }} verb main() -> Int {{ erg total: Int = 0; \
for erg i: u32 in 0u32 .. 2000u32 {{ erg arena: Arena[65536] = Arena[65536](); \
erg first = arena.place(value: Node {{ value: 1, }}); total = total + first.value; }} \
print(total); return 5; }}"
    );
    assert_eq!(run_on_every_backend("arena-loop", &source, "2000\n", 5), BACKENDS_BUILT);
}

#[test]
fn a_value_placed_in_an_arena_is_released_once_with_the_arena() {
    let source = format!(
        "{PRELUDE}verb main() -> Int {{ erg n: Int = 0; loop {{ \
erg arena: Arena[64] = Arena[64](); erg held = arena.place(value: Tracker {{ id: 3, }}); \
n = n + 1; if n > 1 {{ break; }} }} print(9); return 5; }}"
    );
    assert_eq!(run_on_every_backend("arena-placed", &source, "3\n3\n9\n", 5), BACKENDS_BUILT);
}
