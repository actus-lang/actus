//! Arrays are released element by element, last element first.

use super::harness::check;

#[test]
fn an_array_of_buffers_releases_every_element() {
    check(
        "array-buffers",
        "erg a: Buffer = Buffer[1]; erg b: Buffer = Buffer[1]; \
erg slots: Array[Buffer, 2] = Array[Buffer, 2](); slots[0u32] = a; slots[1u32] = b; print(9);",
        "9\n",
    );
}

#[test]
fn an_array_of_drop_structs_runs_the_drop_of_every_element() {
    check(
        "array-trackers",
        "erg trackers: Array[Tracker, 2] = Array[Tracker, 2](); print(9);",
        "9\n0\n0\n",
    );
}

#[test]
fn a_struct_holding_an_array_of_buffers_releases_every_element() {
    check(
        "array-field",
        "erg a: Buffer = Buffer[1]; erg slots: Array[Buffer, 2] = Array[Buffer, 2](); \
slots[1u32] = a; erg shelf: Shelf = Shelf { slots: slots, }; print(9);",
        "9\n",
    );
}
