//! An `Array` of copy types is a copy type (ADR-0124, E1146), so the written
//! `copy(value: abs xs)` accepts it; an array of non-copy elements keeps E1021.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "meta copy struct Stamp { tick: u32, } meta copy enum Level { Low, High, } \
struct Plain { value: Int, } enum Mode { Idle, Busy, } ";

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    analyze(&parse(tokens).expect("source should parse")).map_err(|error| error.kind)
}

#[test]
fn accepts_a_copy_of_an_array_of_copy_types() {
    for element in ["Int", "u8", "Bool", "Stamp", "Level", "Array[u8, 2]", "Array[Stamp, 2]"] {
        let body = format!(
            "erg xs: Array[{element}, 2] = Array[{element}, 2](); \
             erg ys: Array[{element}, 2] = copy(value: abs xs);"
        );
        analyze_main(&body).unwrap_or_else(|error| panic!("{element}: {error:?}"));
    }
}

#[test]
fn keeps_the_original_array_readable_after_its_copy() {
    analyze_main(
        "erg xs: Array[Int, 2] = Array[Int, 2](); erg ys: Array[Int, 2] = copy(value: abs xs); \
         xs[0] = 1; ys[1] = 2;",
    )
    .expect("the copy and the original are independent owners");
}

#[test]
fn rejects_a_copy_of_an_array_of_non_copy_types() {
    for element in ["Buffer", "Plain", "Mode", "Array[Plain, 2]"] {
        let body = format!(
            "erg xs: Array[{element}, 2] = Array[{element}, 2](); \
             erg ys: Array[{element}, 2] = copy(value: abs xs);"
        );
        let error = analyze_main(&body).expect_err(element);
        assert!(
            matches!(&error, SemanticErrorKind::InvalidIntrinsicArgument { callee, .. } if callee == "copy"),
            "{element}: {error:?}"
        );
    }
}

#[test]
fn still_requires_an_abs_readable_array_operand() {
    let error = analyze_main(
        "erg xs: Array[Int, 2] = Array[Int, 2](); erg ys: Array[Int, 2] = copy(value: xs);",
    )
    .expect_err("the operand must be passed as abs");
    assert!(matches!(error, SemanticErrorKind::InvalidArgumentRole { .. }));
}
