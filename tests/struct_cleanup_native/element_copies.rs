//! Bug #123: an array element of a copy type leaves its array only through a
//! written `copy(value: abs xs[i])` (ADR-0124). The copy has storage of its
//! own: changing it does not change the element, and both are released once
//! at their own exits.

use super::harness::run_on_every_backend;

const POINTS: &str = "meta copy struct Point { x: Int, y: Int, } \
meta copy struct Line { start: Point, end: Point, } ";

fn check_copy(label: &str, body: &str, expected_code: i32) {
    let source = format!("{POINTS}verb main() -> Int {{ {body} }}");
    assert_eq!(
        run_on_every_backend(label, &source, "", expected_code),
        super::harness::BACKENDS_BUILT
    );
}

#[test]
fn a_copied_element_has_storage_of_its_own() {
    check_copy(
        "element-copy",
        "erg ps: Array[Point, 2] = Array[Point, 2](); erg i: u32 = 1u32; \
ps[i] = Point { x: 40, y: 2, }; erg p: Point = copy(value: abs ps[i]); p.x = 1; \
return ps[i].x + ps[i].y + p.x - 1;",
        42,
    );
}

#[test]
fn a_copied_field_of_an_element_has_storage_of_its_own() {
    check_copy(
        "element-field-copy",
        "erg ls: Array[Line, 2] = Array[Line, 2](); \
ls[0u32] = Line { start: Point { x: 30, y: 0, }, end: Point { x: 12, y: 0, }, }; \
erg start: Point = copy(value: abs ls[0u32].start); start.x = 0; \
return ls[0u32].start.x + ls[0u32].end.x + start.x;",
        42,
    );
}
