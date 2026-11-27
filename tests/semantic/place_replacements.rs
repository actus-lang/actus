//! Bug #126: an assignment to a field or an array element that holds an owned
//! value releases it before the new value is stored (ADR-0125). The analyzer
//! records the place for the assignment; a field that was moved out releases
//! nothing and becomes owned again. Moving a value that holds a resource out
//! of an array element is rejected (`E1010`): moves of single elements are not
//! tracked, so the array would release the value again.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{OwnershipState, ReplacedPlace, SemanticErrorKind, SemanticModel, analyze};

const PRELUDE: &str = "struct Cfg { erg a: Buffer, erg b: Buffer, } \
struct Inner { erg a: Buffer, erg b: Buffer, } struct Outer { erg inner: Inner, erg z: Buffer, } \
struct Holder { erg buf: Buffer, n: Int, } \
verb sink(dat x: Buffer) { drop(x); } verb sink_inner(dat x: Inner) { drop(x); } \
verb grow(dat x: Buffer) -> Buffer { return x; } verb length(abs x: Buffer) -> Int { return 1; } \
verb hold(erg x: Buffer) {} ";

const CFG: &str = "erg c: Cfg = Cfg { a: Buffer[1], b: Buffer[1], }; ";
const OUTER: &str =
    "erg c: Outer = Outer { inner: Inner { a: Buffer[1], b: Buffer[1], }, z: Buffer[1], }; ";
const BUFFERS: &str =
    "erg xs: Array[Buffer, 2] = Array[Buffer, 2](); erg i: u32 = 0u32; xs[i] = Buffer[1]; ";

fn analyze_main(body: &str) -> Result<SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).map_err(|error| error.kind)
}

fn ownership_of(model: &SemanticModel, name: &str) -> OwnershipState {
    model.bindings.iter().rev().find(|binding| binding.name == name).unwrap().ownership.clone()
}

#[test]
fn an_assignment_to_an_owned_field_records_the_field_it_releases() {
    let model = analyze_main(&format!("{CFG} c.b = Buffer[4];")).expect("accepted");
    let [replacement] = model.place_replacements.as_slice() else {
        panic!("one replacement: {:?}", model.place_replacements)
    };
    assert!(matches!(&replacement.place,
        ReplacedPlace::Field { owner_type, field, moved_fields }
            if owner_type.name == "Cfg" && field == "b" && moved_fields.is_empty()));
    let nested = analyze_main(&format!("{OUTER} c.inner.b = Buffer[4];")).expect("accepted");
    assert!(matches!(&nested.place_replacements[0].place,
        ReplacedPlace::Field { owner_type, field, .. } if owner_type.name == "Inner" && field == "b"));
}

#[test]
fn an_assignment_to_a_plain_field_or_a_scalar_element_records_nothing() {
    let model = analyze_main(
        "erg h: Holder = Holder { buf: Buffer[1], n: 1, }; h.n = 2; \
erg ns: Array[Int, 2] = Array[Int, 2](); ns[0u32] = 3;",
    )
    .expect("accepted");
    assert!(model.place_replacements.is_empty(), "{:?}", model.place_replacements);
}

#[test]
fn an_assignment_to_an_element_that_holds_a_resource_records_the_element() {
    let model = analyze_main(BUFFERS).expect("accepted");
    assert!(matches!(model.place_replacements.as_slice(),
        [replacement] if matches!(&replacement.place,
            ReplacedPlace::Element { element_type } if element_type.name == "Buffer")));
}

#[test]
fn an_assignment_to_a_moved_field_releases_nothing_and_owns_it_again() {
    for body in [
        format!("{CFG} sink(x: dat c.b); c.b = Buffer[4]; erg n: Int = length(x: abs c.b);"),
        format!("{CFG} c.b = grow(x: dat c.b);"),
    ] {
        let model = analyze_main(&body).unwrap_or_else(|error| panic!("{body}: {error:?}"));
        assert!(model.place_replacements.is_empty(), "{body}: {:?}", model.place_replacements);
        assert_eq!(ownership_of(&model, "c"), OwnershipState::Active, "{body}");
    }
}

#[test]
fn an_assignment_keeps_the_other_moved_fields_of_a_partially_moved_owner() {
    let model =
        analyze_main(&format!("{CFG} sink(x: dat c.a); c.b = Buffer[4];")).expect("accepted");
    assert_eq!(model.place_replacements.len(), 1);
    assert_eq!(
        ownership_of(&model, "c"),
        OwnershipState::PartiallyMoved { fields: vec!["a".to_owned()] }
    );
}

#[test]
fn an_assignment_over_a_moved_part_releases_the_rest_of_the_field() {
    let model = analyze_main(&format!(
        "{OUTER} sink(x: dat c.inner.a); c.inner = Inner {{ a: Buffer[2], b: Buffer[2], }};"
    ))
    .expect("accepted");
    assert!(matches!(&model.place_replacements[0].place,
        ReplacedPlace::Field { field, moved_fields, .. }
            if field == "inner" && moved_fields == &vec!["a".to_owned()]));
    assert_eq!(ownership_of(&model, "c"), OwnershipState::Active);
}

#[test]
fn an_assignment_into_a_moved_value_is_rejected() {
    let error =
        analyze_main(&format!("{OUTER} sink_inner(x: dat c.inner); c.inner.a = Buffer[2];"))
            .expect_err("the field holds no value to assign into");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "c"), "{error:?}");
}

#[test]
fn moving_a_resource_out_of_an_array_element_is_rejected() {
    for (label, tail, path) in [
        ("binding", "erg y: Buffer = xs[i];", "xs[..]"),
        ("literal index", "erg y: Buffer = xs[0u32];", "xs[0u32]"),
        ("dat argument", "sink(x: dat xs[i]);", "xs[..]"),
        ("erg argument", "hold(x: xs[i]);", "xs[..]"),
        ("struct field", "erg h: Holder = Holder { buf: xs[i], n: 1, };", "xs[..]"),
        ("enum payload", "erg o: Option[Buffer] = Option[Buffer].Some(xs[i]);", "xs[..]"),
        ("assignment", "erg y: Buffer = Buffer[1]; y = xs[i];", "xs[..]"),
        (
            "field assignment",
            "erg h: Holder = Holder { buf: Buffer[1], n: 1, }; h.buf = xs[i];",
            "xs[..]",
        ),
        ("element", "erg ys: Array[Buffer, 2] = Array[Buffer, 2](); ys[0u32] = xs[i];", "xs[..]"),
        ("branch value", "erg y: Buffer = if i == 0u32 { xs[i] } else { Buffer[1] };", "xs[..]"),
        (
            "field of an element",
            "erg hs: Array[Holder, 2] = Array[Holder, 2](); erg y: Buffer = hs[i].buf;",
            "hs[..].buf",
        ),
    ] {
        let error = analyze_main(&format!("{BUFFERS} {tail}")).expect_err(label);
        match &error {
            SemanticErrorKind::MoveOutOfElement(found) => assert_eq!(found, path, "{label}"),
            _ => panic!("{label}: {error:?}"),
        }
    }
}

#[test]
fn reading_an_element_by_view_or_a_scalar_element_stays_accepted() {
    analyze_main(&format!(
        "{BUFFERS} erg n: Int = length(x: abs xs[i]); \
erg ns: Array[Int, 2] = Array[Int, 2](); erg m: Int = ns[i]; erg b: u8 = xs[i][0u32];"
    ))
    .expect("a view of an element and a scalar element are not moved");
}

#[test]
fn moving_out_of_an_element_is_reported_as_e1010_with_the_path() {
    let source = format!("{PRELUDE}verb main() {{ {BUFFERS} erg y: Buffer = xs[i]; }}");
    let (tokens, _) = scan(&source);
    let error = analyze(&parse(tokens).expect("parse")).expect_err("rejected");
    let rendered = actus::diagnostics::semantic_diagnostic(&error);
    assert_eq!(rendered.code(), "E1010");
    assert!(rendered.message().contains("`xs[..]`"), "{}", rendered.message());
    assert!(rendered.message().contains("borrow it with `abs`"), "{}", rendered.message());
}
