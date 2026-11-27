//! Bug #131: the value of an `if` whose branches read a struct field had no
//! type, so `erg x: Plain = if c { .. } else { .. };` reported `E1027` with
//! `found unknown`. The type of an `if` value is the type of its branch
//! values, and a mismatch names the real types. The reads follow the
//! ownership rules (ADR-0124): a copy type is duplicated with a written
//! `copy`, an owner moves its own field, and a scalar field is read.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::analyze;

const PRELUDE: &str = "meta copy struct Plain { kind: u8, } struct Other { kind: u8, } \
struct Rec { erg p: Plain, n: Int, } ";
const REC: &str = "erg r: Rec = Rec { p: Plain { kind: 1u8, }, n: 2, }; erg c: Bool = true; ";

fn analyze_main(body: &str) -> Result<(), (String, String)> {
    let source = format!("{PRELUDE}verb main() {{ {REC}{body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map(|_| ()).map_err(|error| {
        let rendered = actus::diagnostics::semantic_diagnostic(&error);
        (rendered.code().to_owned(), rendered.message().to_owned())
    })
}

#[test]
fn an_if_value_of_struct_fields_has_the_type_of_its_branches() {
    for (label, body) in [
        (
            "written copy",
            "erg x: Plain = if c { copy(value: abs r.p) } else { copy(value: abs r.p) };",
        ),
        ("moved field", "erg x: Plain = if c { r.p } else { r.p };"),
        ("else if", "erg x: Plain = if c { r.p } else if r.n > 1 { r.p } else { r.p };"),
        ("scalar field", "erg n: Int = if c { r.n } else { r.n };"),
    ] {
        if let Err((code, message)) = analyze_main(body) {
            panic!("{label}: expected acceptance, found {code}: {message}");
        }
    }
}

#[test]
fn a_mismatched_if_value_names_the_real_types() {
    let (code, message) =
        analyze_main("erg x: Other = if c { copy(value: abs r.p) } else { copy(value: abs r.p) };")
            .expect_err("a Plain value is not an Other");
    assert_eq!(code, "E1027", "{message}");
    assert!(message.contains("expected `Other`, found `Plain`"), "{message}");
    assert!(!message.contains("unknown"), "{message}");
}
