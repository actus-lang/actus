use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Node { value: Int, } struct Inner { number: Int, } \
struct PlainArena { held: Arena[64], } struct OwnedArena { erg held: Arena[64], } \
struct PlainBuffer { held: Buffer, } struct OwnedBuffer { erg held: Buffer, } \
struct PlainStruct { held: Inner, } struct OwnedStruct { erg held: Inner, } \
struct PlainCount { held: Int, } \
verb take(dat arena: Arena[64]) {} ";

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

const ARENA: &str = "erg source: Arena[64] = Arena[64](); ";
const BUFFER: &str = "erg source: Buffer = Buffer[4]; ";
const STRUCT: &str = "erg source = Inner { number: 1, }; ";

#[test]
fn a_plain_field_moves_every_move_only_source_like_an_erg_field() {
    for (label, declare, plain, owned, reuse) in [
        (
            "arena",
            ARENA,
            "PlainArena { held: source, }",
            "OwnedArena { held: source, }",
            "erg again = source.place(value: Node { value: 1, });",
        ),
        (
            "arena drop",
            ARENA,
            "PlainArena { held: source, }",
            "OwnedArena { held: source, }",
            "drop(source);",
        ),
        (
            "arena handed to a call",
            ARENA,
            "PlainArena { held: source, }",
            "OwnedArena { held: source, }",
            "take(arena: source);",
        ),
        (
            "buffer append",
            BUFFER,
            "PlainBuffer { held: source, }",
            "OwnedBuffer { held: source, }",
            "append(source, 1u8);",
        ),
        (
            "buffer drop",
            BUFFER,
            "PlainBuffer { held: source, }",
            "OwnedBuffer { held: source, }",
            "drop(source);",
        ),
        (
            "struct",
            STRUCT,
            "PlainStruct { held: source, }",
            "OwnedStruct { held: source, }",
            "print(source.number);",
        ),
    ] {
        let plain_error = analyze_main(&format!("{declare}erg holder = {plain}; {reuse}"))
            .expect_err(&format!("{label}: a plain field must consume its source"));
        let owned_error = analyze_main(&format!("{declare}erg holder = {owned}; {reuse}"))
            .expect_err(&format!("{label}: an erg field must consume its source"));
        assert!(
            matches!(&plain_error, SemanticErrorKind::UseAfterMove { name }
                | SemanticErrorKind::DoubleDrop { name } if name == "source"),
            "{label}: {plain_error:?}"
        );
        assert_eq!(plain_error, owned_error, "{label}: plain and erg fields must agree");
    }
}

#[test]
fn a_plain_field_that_takes_an_arena_follows_the_arena_release_rules() {
    let placed =
        "erg source: Arena[64] = Arena[64](); erg node = source.place(value: Node { value: 1, }); ";
    let live = analyze_main(&format!(
        "{placed}abs view = ref node; erg holder = PlainArena {{ held: source, }}; print(view.value);"
    ))
    .expect_err("an arena cannot move into a field while a view is live");
    assert!(matches!(live, SemanticErrorKind::ArenaReferenceLive { arena, reference }
        if arena == "source" && reference == "view"));
    let released = analyze_main(&format!(
        "{placed}erg holder = PlainArena {{ held: source, }}; print(node.value);"
    ))
    .expect_err("a placed value cannot be read once its arena moved into a field");
    assert!(matches!(released, SemanticErrorKind::ArenaReferenceReleased { arena, reference }
        if arena == "source" && reference == "node"));
}

#[test]
fn a_plain_field_rejects_a_source_that_cannot_be_moved() {
    for (label, params, body, expected) in [
        ("ins arena", "ins source: Arena[64]", "PlainArena { held: source, }", "EscapingLoan"),
        (
            "abs arena",
            "abs source: Arena[64]",
            "PlainArena { held: source, }",
            "InvalidOwnerInitializer",
        ),
        ("ins buffer", "ins source: Buffer", "PlainBuffer { held: source, }", "EscapingLoan"),
        (
            "abs buffer",
            "abs source: Buffer",
            "PlainBuffer { held: source, }",
            "InvalidOwnerInitializer",
        ),
    ] {
        let source = format!("{PRELUDE}verb borrow({params}) {{ erg holder = {body}; }}");
        let (tokens, _) = scan(&source);
        let error = analyze(&parse(tokens).expect("source should parse")).expect_err(label).kind;
        assert!(format!("{error:?}").starts_with(expected), "{label}: {error:?}");
    }
}

#[test]
fn a_plain_field_moves_the_erg_field_it_is_built_from() {
    for (label, declare, plain, reuse) in [
        ("buffer", BUFFER, "PlainBuffer { held: owner.held, }", "append(owner.held, 1u8);"),
        ("struct", STRUCT, "PlainStruct { held: owner.held, }", "print(owner.held.number);"),
    ] {
        let owner = if label == "buffer" {
            "erg owner = OwnedBuffer { held: source, };"
        } else {
            "erg owner = OwnedStruct { held: source, };"
        };
        let error = analyze_main(&format!("{declare}{owner} erg holder = {plain}; {reuse}"))
            .expect_err(label);
        assert!(
            matches!(error, SemanticErrorKind::UseAfterMove { .. }),
            "{label}: the source field must be moved, got {error:?}"
        );
    }
}

#[test]
fn a_plain_field_cannot_move_a_resource_out_of_another_plain_field() {
    let error = analyze_main(&format!(
        "{BUFFER}erg owner = PlainBuffer {{ held: source, }}; erg other = PlainBuffer {{ held: owner.held, }};"
    ))
    .expect_err("a plain field is not a movable place");
    assert!(matches!(error, SemanticErrorKind::InvalidDatArgument { .. }), "{error:?}");
}

#[test]
fn a_plain_field_still_accepts_fresh_values_and_scalar_sources() {
    for (label, body) in [
        ("fresh arena", "erg holder = PlainArena { held: Arena[64](), }; print(1);"),
        ("fresh buffer", "erg holder = PlainBuffer { held: Buffer[4], }; print(1);"),
        (
            "owner used through its holder",
            "erg source: Arena[64] = Arena[64](); erg holder = PlainArena { held: source, }; print(1);",
        ),
        (
            "scalar source stays readable",
            "erg count: Int = 1; erg holder = PlainCount { held: count, }; print(count);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}
