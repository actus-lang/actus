//! An `abs` struct field is a read-only view of its source (ADR-0046 2.3). The
//! source is frozen for as long as the holder lives, which is the end of the
//! block that builds it, exactly as for an `abs` binding (ADR-0016). It cannot
//! be moved, dropped, or mutated while the view exists.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Plain { number: Int, } struct ViewBuffer { abs held: Buffer, } \
struct ViewPlain { abs held: Plain, } struct OwnPlain { erg inner: Plain, } \
struct Outer { erg view: ViewBuffer, } struct ViewCount { abs held: Int, } \
verb consume(dat held: Buffer) {} ";

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

const BUFFER: &str = "erg source: Buffer = Buffer[4]; ";
const PLAIN: &str = "erg plain = Plain { number: 1, }; ";

#[test]
fn an_abs_field_freezes_its_source_while_the_holder_lives() {
    for (label, declare, build, action, expected) in [
        ("drop", BUFFER, "ViewBuffer { held: source, }", "drop(source);", "DropFrozen"),
        (
            "drop explicit ref",
            BUFFER,
            "ViewBuffer { held: ref source, }",
            "drop(source);",
            "DropFrozen",
        ),
        (
            "move to a binding",
            BUFFER,
            "ViewBuffer { held: source, }",
            "erg other = source;",
            "MoveFrozen",
        ),
        (
            "move to a dat call",
            BUFFER,
            "ViewBuffer { held: source, }",
            "consume(held: source);",
            "MoveFrozen",
        ),
        (
            "mutate a field",
            PLAIN,
            "ViewPlain { held: plain, }",
            "plain.number = 5;",
            "FieldBorrowConflict",
        ),
        (
            "move a whole struct",
            PLAIN,
            "ViewPlain { held: plain, }",
            "erg other = plain;",
            "MoveFrozen",
        ),
        (
            "nested literal",
            BUFFER,
            "Outer { view: ViewBuffer { held: source, }, }",
            "drop(source);",
            "DropFrozen",
        ),
        (
            "assigned later",
            BUFFER,
            "ViewBuffer { held: source, }",
            "erg again = source;",
            "MoveFrozen",
        ),
    ] {
        let error = analyze_main(&format!("{declare}erg holder = {build}; {action}"))
            .expect_err(&format!("{label}: the source must stay frozen"));
        assert!(format!("{error:?}").starts_with(expected), "{label}: {error:?}");
    }
}

#[test]
fn an_abs_field_freezes_the_owner_of_the_field_it_is_built_from() {
    let error = analyze_main(
        "erg owner = OwnPlain { inner: Plain { number: 1, }, }; erg holder = ViewPlain { held: owner.inner, }; drop(owner);",
    )
    .expect_err("a field view freezes its owner");
    assert!(matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "owner"));
}

#[test]
fn an_abs_field_view_ends_with_the_block_of_its_holder() {
    for (label, body) in [
        (
            "inner block",
            "erg source: Buffer = Buffer[4]; { erg holder = ViewBuffer { held: source, }; } drop(source);",
        ),
        (
            "inner block then move",
            "erg source: Buffer = Buffer[4]; { erg holder = ViewBuffer { held: source, }; } erg other = source; drop(other);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn an_abs_field_still_allows_reading_and_sharing_the_source() {
    for (label, body) in [
        (
            "read through the owner",
            "erg plain = Plain { number: 1, }; erg holder = ViewPlain { held: plain, }; print(plain.number); print(holder.held.number);",
        ),
        (
            "two holders",
            "erg plain = Plain { number: 1, }; erg first = ViewPlain { held: plain, }; erg second = ViewPlain { held: plain, }; print(plain.number);",
        ),
        ("fresh value", "erg holder = ViewPlain { held: Plain { number: 1, }, }; print(1);"),
        ("scalar", "erg count: Int = 1; erg holder = ViewCount { held: count, }; print(count);"),
        (
            "drop after the last use of the block",
            "erg source: Buffer = Buffer[4]; erg holder = ViewBuffer { held: source, }; print(1);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn an_abs_field_passes_on_a_view_and_rejects_a_stored_loan() {
    let source = format!(
        "{PRELUDE}verb forward(abs source: Buffer) {{ erg holder = ViewBuffer {{ held: source, }}; }} verb main() {{ }}"
    );
    let (tokens, _) = scan(&source);
    analyze(&parse(tokens).expect("source should parse"))
        .expect("an abs parameter is already a view");
    let source = format!(
        "{PRELUDE}verb store(ins source: Buffer) {{ erg holder = ViewBuffer {{ held: source, }}; }} verb main() {{ }}"
    );
    let (tokens, _) = scan(&source);
    let error = analyze(&parse(tokens).expect("source should parse"))
        .expect_err("a call-scope loan cannot be stored in a field")
        .kind;
    assert!(matches!(error, SemanticErrorKind::EscapingLoan { name } if name == "source"));
}

#[test]
fn an_abs_enum_payload_freezes_its_source_like_an_abs_field() {
    for (label, construct) in [
        ("bare", "Option[abs Buffer].Some(source)"),
        ("explicit ref", "Option[abs Buffer].Some(ref source)"),
    ] {
        let error = analyze_main(&format!("{BUFFER}erg holder = {construct}; drop(source);"))
            .expect_err(label);
        assert!(
            matches!(error, SemanticErrorKind::DropFrozen { name, .. } if name == "source"),
            "{label}"
        );
    }
    analyze_main(&format!("{BUFFER}erg holder = Option[abs Buffer].Some(source); print(1);"))
        .expect("an abs payload leaves its source alive");
    analyze_main(&format!(
        "{BUFFER}{{ erg holder = Option[abs Buffer].Some(source); }} drop(source);"
    ))
    .expect("the view ends with the holder's block");
}
