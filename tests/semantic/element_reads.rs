//! Bug #123: reading an array element of a struct, enum, pack or array type BY
//! VALUE is a move out of an indexed element, which the language does not
//! track, so it is rejected with `E1010` for every aggregate type, not only for
//! the types that hold a resource (ADR-0124). A copy type is duplicated only by
//! a written `copy(value: abs xs[i])`; a scalar element stays a value, and a
//! read by view (`abs xs[i]`) never moves.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

const PRELUDE: &str = "meta copy struct Point { x: Int, y: Int, } \
struct Plain { kind: u8, } \
meta copy pack Control { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } \
enum Mode { Idle, Busy, } \
struct Box { erg point: Point, erg plain: Plain, } \
verb take_point(erg p: Point) {} verb look(abs p: Plain) -> u8 { return p.kind; } ";

/// Element array declarations: binding, element type and whether
/// `copy(value: abs ...)` is the fix.
const ARRAYS: [(&str, &str, bool); 5] = [
    ("ps", "Point", true),
    ("qs", "Plain", false),
    ("cs", "Control", true),
    ("ms", "Mode", false),
    ("rows", "Array[Point, 2]", true),
];

fn declarations() -> String {
    let mut body = String::from("erg i: u32 = 0u32; ");
    for (name, element, _) in ARRAYS {
        body.push_str(&format!("erg {name}: Array[{element}, 2] = Array[{element}, 2](); "));
    }
    body.push_str("erg bs: Array[Box, 2] = Array[Box, 2](); ");
    body
}

fn diagnostic(body: &str) -> Result<(String, String, String), ()> {
    let source = format!("{PRELUDE}verb main() {{ {} {body} }}", declarations());
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let Err(error) = analyze(&program) else { return Err(()) };
    let rendered = actus::diagnostics::semantic_diagnostic(&error);
    let span = rendered.span();
    Ok((
        rendered.code().to_owned(),
        rendered.message().to_owned(),
        source[span.start..span.end].to_owned(),
    ))
}

/// Asserts that `body` is rejected with `E1010` naming `path`, with the
/// written copy as the fix exactly when the element type is a copy type.
fn rejected(label: &str, body: &str, path: &str, copyable: bool) {
    let (code, message, spanned) =
        diagnostic(body).unwrap_or_else(|()| panic!("{label}: `{body}` must be rejected"));
    assert_eq!(code, "E1010", "{label}: {message}");
    assert!(message.contains(&format!("`{path}`")), "{label}: {message}");
    assert!(message.contains("out of an array element"), "{label}: {message}");
    let written = path.replace("[..]", "[i]");
    assert!(spanned.contains(&written), "{label}: span `{spanned}`");
    let fix = format!("copy(value: abs {path})");
    assert_eq!(message.contains(&fix), copyable, "{label}: {message}");
}

fn accepted(label: &str, body: &str) {
    if let Ok((code, message, _)) = diagnostic(body) {
        panic!("{label}: `{body}` must be accepted, found {code}: {message}");
    }
}

#[test]
fn a_by_value_element_read_of_every_aggregate_type_is_rejected_in_a_binding() {
    for (name, element, copyable) in ARRAYS {
        let body = format!("erg v: {element} = {name}[i];");
        rejected(element, &body, &format!("{name}[..]"), copyable);
    }
}

#[test]
fn a_by_value_element_read_is_rejected_in_every_position() {
    for (label, body, path) in [
        ("binding", "erg p: Point = ps[0u32];", "ps[0u32]"),
        ("assignment", "erg p: Point = Point { x: 1, y: 2, }; p = ps[i];", "ps[..]"),
        ("argument", "take_point(p: ps[i]);", "ps[..]"),
        (
            "field initialiser",
            "erg b: Box = Box { point: ps[i], plain: Plain { kind: 1u8, }, };",
            "ps[..]",
        ),
        ("payload", "erg o: Option[Point] = Option[Point].Some(ps[i]);", "ps[..]"),
        ("element", "ps[1u32] = ps[i];", "ps[..]"),
        (
            "branch value",
            "erg p: Point = if i == 0u32 { ps[i] } else { Point { x: 0, y: 0, } };",
            "ps[..]",
        ),
        ("field of an element", "erg p: Point = bs[i].point;", "bs[..].point"),
    ] {
        rejected(label, body, path, true);
    }
}

#[test]
fn a_by_value_element_read_is_rejected_in_a_return() {
    let source = format!(
        "{PRELUDE}verb first(abs ps: Array[Point, 2]) -> Point {{ return copy(value: abs ps[0u32]); }} \
verb own() -> Plain {{ erg qs: Array[Plain, 2] = Array[Plain, 2](); return qs[0u32]; }} verb main() {{ }}"
    );
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let error = analyze(&parse(tokens).expect("parse")).expect_err("the return moves an element");
    let rendered = actus::diagnostics::semantic_diagnostic(&error);
    assert_eq!(rendered.code(), "E1010", "{}", rendered.message());
    assert!(rendered.message().contains("`qs[0u32]`"), "{}", rendered.message());
}

#[test]
fn an_explicit_copy_a_view_and_a_scalar_element_stay_accepted() {
    accepted("explicit copy", "erg p: Point = copy(value: abs ps[i]);");
    accepted("explicit copy of a field", "erg p: Point = copy(value: abs bs[i].point);");
    accepted("copy of a pack", "erg c: Control = copy(value: abs cs[i]);");
    accepted("copy of an array", "erg r: Array[Point, 2] = copy(value: abs rows[i]);");
    accepted("view", "erg k: u8 = look(p: abs qs[i]);");
    accepted("scalar field", "erg x: Int = ps[i].x;");
    accepted("scalar element", "erg ns: Array[Int, 2] = Array[Int, 2](); erg n: Int = ns[i];");
}
