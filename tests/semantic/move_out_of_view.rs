//! Bug #108: a struct, enum, pack or array read by value through an `abs` view
//! or an `ins` loan is neither moved (a view cannot give up ownership) nor
//! copied (ADR-0124), so it was shared silently. Each read site is rejected
//! with the code of a value that cannot leave a view (`E1010` for `abs`,
//! `E1082` for an `ins` loan), and the message names the path and the fix.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "meta copy struct Header { kind: u8, length: u16, } \
struct Plain { kind: u8, } \
meta copy pack Control { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } \
enum Mode { Idle, Busy, } \
struct Parser { erg header: Header, erg plain: Plain, erg control: Control, erg mode: Mode, \
erg items: Array[Plain, 2], erg stamps: Array[Header, 2], erg buf: Buffer, tag: Header, n: Int, } \
struct Outer { erg inner: Parser, } ";

/// Field, its type and whether `copy(value: abs ...)` is the fix.
const FIELDS: [(&str, &str, bool); 7] = [
    ("header", "Header", true),
    ("plain", "Plain", false),
    ("control", "Control", true),
    ("mode", "Mode", false),
    ("items", "Array[Plain, 2]", false),
    ("stamps", "Array[Header, 2]", true),
    ("buf", "Buffer", false),
];

fn diagnostic(source: &str) -> (String, String, String) {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("source must be rejected");
    let rendered = actus::diagnostics::semantic_diagnostic(&error);
    let span = rendered.span();
    (
        rendered.code().to_owned(),
        rendered.message().to_owned(),
        source[span.start..span.end].to_owned(),
    )
}

fn accepted(body: &str) {
    let source = format!("{PRELUDE}{body} verb main() {{ }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).expect("source must be accepted");
}

/// Asserts that `body` is rejected as a move out of the view `root` and that
/// the message names `path`.
fn rejected(label: &str, body: &str, code: &str, path: &str, copyable: bool) {
    let source = format!("{PRELUDE}{body} verb main() {{ }}");
    let (found, message, spanned) = diagnostic(&source);
    assert_eq!(found, code, "{label}: {message}");
    assert!(message.contains(&format!("`{path}`")), "{label}: {message}");
    assert!(spanned.contains(path), "{label}: span `{spanned}`");
    if copyable {
        assert!(message.contains(&format!("copy(value: abs {path})")), "{label}: {message}");
    } else {
        assert!(message.contains("borrow it with `abs`"), "{label}: {message}");
        assert!(!message.contains("copy(value"), "{label}: {message}");
    }
}

/// Runs `template` for every field type. `{f}` is the field, `{t}` its type.
fn for_every_field(site: &str, template: &str, view: &str, code: &str) {
    for (field, type_name, copyable) in FIELDS {
        let body = template.replace("{f}", field).replace("{t}", type_name);
        rejected(&format!("{site} {field}"), &body, code, &format!("{view}.{field}"), copyable);
    }
}

#[test]
fn returning_a_field_through_an_abs_view_is_rejected() {
    for_every_field("return", "verb get(abs p: Parser) -> {t} { return p.{f}; }", "p", "E1010");
}

#[test]
fn initializing_an_owner_from_a_field_through_an_abs_view_is_rejected() {
    for_every_field("erg init", "verb get(abs p: Parser) { erg x = p.{f}; }", "p", "E1010");
    for_every_field("typed init", "verb get(abs p: Parser) { erg x: {t} = p.{f}; }", "p", "E1010");
}

#[test]
fn passing_a_field_through_an_abs_view_to_a_dat_parameter_is_rejected() {
    for_every_field(
        "dat argument",
        "verb take(dat v: {t}) { } verb get(abs p: Parser) { take(v: dat p.{f}); }",
        "p",
        "E1010",
    );
}

#[test]
fn passing_a_field_through_an_abs_view_to_an_erg_parameter_is_rejected() {
    // An `erg` argument is refused earlier, as a role that the view cannot satisfy.
    let body = "verb take(erg v: Header) { } verb get(abs p: Parser) { take(v: p.header); }";
    let (code, message, _) = diagnostic(&format!("{PRELUDE}{body} verb main() {{ }}"));
    assert_eq!(code, "E1016", "{message}");
}

#[test]
fn storing_a_field_through_an_abs_view_in_a_struct_literal_is_rejected() {
    for_every_field(
        "struct literal",
        "struct Wrapper { erg held: {t}, } verb get(abs p: Parser) -> Wrapper { return Wrapper { held: p.{f}, }; }",
        "p",
        "E1010",
    );
}

#[test]
fn storing_a_field_through_an_abs_view_in_an_enum_payload_is_rejected() {
    for_every_field(
        "enum payload",
        "verb get(abs p: Parser) -> Option[{t}] { return Option[{t}].Some(p.{f}); }",
        "p",
        "E1010",
    );
}

#[test]
fn assigning_a_field_through_an_abs_view_is_rejected() {
    rejected(
        "local assignment",
        "verb get(abs p: Parser) { erg y = Header { kind: 1u8, length: 2u16, }; y = p.header; }",
        "E1010",
        "p.header",
        true,
    );
    rejected(
        "field assignment",
        "verb get(abs p: Parser, ins q: Parser) { q.header = p.header; }",
        "E1010",
        "p.header",
        true,
    );
}

#[test]
fn storing_an_array_field_through_an_abs_view_in_a_slot_is_rejected() {
    rejected(
        "field assignment",
        "verb get(abs p: Parser, ins q: Parser) { q.items = p.items; }",
        "E1010",
        "p.items",
        false,
    );
    rejected(
        "element assignment",
        "verb get(abs p: Parser, ins q: Parser) { q.items[0] = p.plain; }",
        "E1010",
        "p.plain",
        false,
    );
    rejected(
        "array assignment",
        "verb get(abs p: Parser) { erg y: Array[Header, 2] = Array[Header, 2](); y = p.stamps; }",
        "E1010",
        "p.stamps",
        true,
    );
}

#[test]
fn a_nested_path_names_the_whole_path() {
    rejected(
        "nested return",
        "verb get(abs o: Outer) -> Header { return o.inner.header; }",
        "E1010",
        "o.inner.header",
        true,
    );
    rejected(
        "nested init",
        "verb get(abs o: Outer) { erg x = o.inner.plain; }",
        "E1010",
        "o.inner.plain",
        false,
    );
}

#[test]
fn a_value_role_field_is_rejected_with_the_same_message() {
    rejected(
        "value field",
        "verb get(abs p: Parser) -> Header { return p.tag; }",
        "E1010",
        "p.tag",
        true,
    );
    rejected(
        "value field init",
        "verb get(abs p: Parser) { erg x = p.tag; }",
        "E1010",
        "p.tag",
        true,
    );
}

#[test]
fn an_aggregate_element_of_a_view_is_rejected() {
    rejected(
        "element return",
        "verb get(abs p: Parser) -> Plain { return p.items[0]; }",
        "E1010",
        "p.items[0]",
        false,
    );
    rejected(
        "element init",
        "verb get(abs p: Parser) { erg x = p.items[0]; }",
        "E1010",
        "p.items[0]",
        false,
    );
    rejected(
        "copy element",
        "verb get(abs p: Parser) { erg x = p.stamps[1]; }",
        "E1010",
        "p.stamps[1]",
        true,
    );
}

#[test]
fn a_field_through_an_ins_loan_is_rejected_as_an_escaping_loan() {
    for_every_field("ins return", "verb get(ins p: Parser) -> {t} { return p.{f}; }", "p", "E1082");
    for_every_field("ins init", "verb get(ins p: Parser) { erg x = p.{f}; }", "p", "E1082");
    rejected(
        "ins element",
        "verb get(ins p: Parser) { erg x = p.items[0]; }",
        "E1082",
        "p.items[0]",
        false,
    );
}

#[test]
fn moving_a_field_out_of_an_abs_view_reports_the_view_not_a_moved_binding() {
    // Bug #113: the field was recorded as moved out of the view and the next
    // use of the view then reported `E1005`.
    let body = "struct Action { erg kind: Header, } struct Event { erg kind: Header, } \
        verb relay(abs action: Action) -> Event { return Event { kind: action.kind, }; }";
    rejected("literal field", body, "E1010", "action.kind", true);
    let body = "struct Action { erg kind: Header, n: Int, } struct Event { erg kind: Header, n: Int, } \
        verb relay(abs action: Action) -> Event { erg event = Event { kind: action.kind, n: action.n, }; return event; }";
    rejected("literal field and later use", body, "E1010", "action.kind", true);
}

#[test]
fn the_error_kind_records_the_path_and_the_fix() {
    let source = format!("{PRELUDE}verb get(abs p: Parser) -> Header {{ return p.header; }}");
    let (tokens, _) = scan(&source);
    let error = analyze(&parse(tokens).expect("parse")).expect_err("rejected");
    assert_eq!(
        error.kind,
        SemanticErrorKind::MoveOutOfView {
            path: "p.header".to_owned(),
            loan: false,
            copyable: true
        }
    );
}

#[test]
fn a_written_copy_of_a_copy_type_field_through_a_view_is_accepted() {
    accepted("verb get(abs p: Parser) -> Header { return copy(value: abs p.header); }");
    accepted("verb get(abs p: Parser) { erg x = copy(value: abs p.header); }");
    accepted("verb get(abs p: Parser) { erg x = copy(value: abs p.control); }");
    accepted("verb get(abs p: Parser) { erg x = copy(value: abs p.stamps); }");
    accepted("verb get(abs p: Parser) { erg x = copy(value: abs p.stamps[0]); }");
    accepted("verb get(abs o: Outer) -> Header { return copy(value: abs o.inner.header); }");
    accepted("verb get(ins p: Parser) -> Header { return copy(value: abs p.header); }");
}

#[test]
fn scalar_fields_through_a_view_are_unchanged() {
    accepted("verb get(abs p: Parser) -> Int { return p.n; }");
    accepted("verb get(abs p: Parser) -> u8 { return p.header.kind; }");
    accepted("verb get(abs p: Parser) { erg x = p.n; erg y: u16 = p.header.length; }");
    accepted("verb get(ins p: Parser) -> Int { return p.n; }");
}

#[test]
fn taking_a_view_of_a_field_is_unchanged() {
    accepted("verb get(abs p: Parser) { abs h = ref p.header; }");
    accepted("verb show(abs h: Header) { } verb get(abs p: Parser) { show(h: abs p.header); }");
    accepted("verb get(abs p: Parser) -> abs Header { return p.header; }");
    accepted(
        "verb get(abs p: Parser) -> Int { return case abs p.mode { Mode.Idle => 1, Mode.Busy => 2, }; }",
    );
    accepted("verb get(abs p: Parser) -> Bool { return p.mode == Mode.Idle; }");
}

#[test]
fn an_owner_still_moves_a_field_out_of_itself() {
    accepted("verb get(erg p: Parser) -> Header { return p.header; }");
    accepted("verb get(erg p: Parser) { erg x = p.header; }");
    accepted("verb take(dat v: Header) { } verb get(erg p: Parser) { take(v: dat p.header); }");
    accepted("verb get(dat p: Parser) -> Plain { return p.plain; }");
}
