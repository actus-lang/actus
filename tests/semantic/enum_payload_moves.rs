//! A move-only value placed in an enum payload moves into the enum value
//! (ADR-0016, ADR-0030, ADR-0059): the source cannot stay behind as a second
//! owner of the same buffer or arena. `Option` and `Result` follow the same
//! rule as user enums.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Node { value: Int, } struct Inner { number: Int, } \
struct HoldBuffer { erg held: Buffer, } \
enum Mine { Arena(Arena[64]), Buffer(Buffer), Array(Array[Buffer, 2]), \
Wrapped(Option[Buffer]), Named { held: Arena[64], }, Count(Int), } ";

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
const ARRAY: &str = "erg source: Array[Buffer, 2] = Array[Buffer, 2](); ";
const WRAPPED: &str = "erg source: Option[Buffer] = Option[Buffer].None; ";

#[test]
fn a_payload_moves_every_move_only_source_into_the_enum_value() {
    for (label, declare, construct, reuse) in [
        (
            "option arena",
            ARENA,
            "Option[Arena[64]].Some(source)",
            "erg again = source.place(value: Node { value: 1, });",
        ),
        ("option arena drop", ARENA, "Option[Arena[64]].Some(source)", "drop(source);"),
        ("option buffer", BUFFER, "Option[Buffer].Some(source)", "append(source, 1u8);"),
        ("option struct", STRUCT, "Option[Inner].Some(source)", "print(source.number);"),
        ("option array", ARRAY, "Option[Array[Buffer, 2]].Some(source)", "drop(source);"),
        ("option enum", WRAPPED, "Option[Option[Buffer]].Some(source)", "drop(source);"),
        ("result ok arena", ARENA, "Result[Arena[64], Int].Ok(source)", "drop(source);"),
        ("result err arena", ARENA, "Result[Int, Arena[64]].Err(source)", "drop(source);"),
        ("result ok buffer", BUFFER, "Result[Buffer, Int].Ok(source)", "append(source, 1u8);"),
        ("user arena", ARENA, "Mine.Arena(source)", "drop(source);"),
        ("user buffer", BUFFER, "Mine.Buffer(source)", "drop(source);"),
        ("user array", ARRAY, "Mine.Array(source)", "drop(source);"),
        ("user enum payload", WRAPPED, "Mine.Wrapped(source)", "drop(source);"),
        ("user named arena", ARENA, "Mine.Named(held: source)", "drop(source);"),
        ("explicit dat", BUFFER, "Option[Buffer].Some(dat source)", "drop(source);"),
    ] {
        let error = analyze_main(&format!("{declare}erg holder = {construct}; {reuse}"))
            .expect_err(&format!("{label}: the payload must consume its source"));
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name }
                | SemanticErrorKind::DoubleDrop { name } if name == "source"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn a_payload_that_takes_an_arena_follows_the_arena_release_rules() {
    let placed =
        "erg source: Arena[64] = Arena[64](); erg node = source.place(value: Node { value: 1, }); ";
    let live = analyze_main(&format!(
        "{placed}abs view = ref node; erg holder = Option[Arena[64]].Some(source); print(view.value);"
    ))
    .expect_err("an arena cannot move into a payload while a view is live");
    assert!(matches!(live, SemanticErrorKind::ArenaReferenceLive { arena, reference }
        if arena == "source" && reference == "view"));
    let released = analyze_main(&format!(
        "{placed}erg holder = Option[Arena[64]].Some(source); print(node.value);"
    ))
    .expect_err("a placed value cannot be read once its arena moved into a payload");
    assert!(matches!(released, SemanticErrorKind::ArenaReferenceReleased { arena, reference }
        if arena == "source" && reference == "node"));
}

#[test]
fn a_payload_moves_the_erg_field_it_is_built_from() {
    for (label, construct) in [
        ("option", "Option[Buffer].Some(owner.held)"),
        ("user enum", "Mine.Buffer(owner.held)"),
        ("result", "Result[Buffer, Int].Ok(owner.held)"),
    ] {
        let error = analyze_main(&format!(
            "{BUFFER}erg owner = HoldBuffer {{ held: source, }}; erg holder = {construct}; append(owner.held, 1u8);"
        ))
        .expect_err(label);
        assert!(
            matches!(error, SemanticErrorKind::UseAfterMove { .. }),
            "{label}: the source field must be moved, got {error:?}"
        );
    }
}

#[test]
fn a_payload_rejects_a_source_that_cannot_be_moved() {
    for (label, params, construct, expected) in [
        ("ins arena", "ins source: Arena[64]", "Option[Arena[64]].Some(source)", "EscapingLoan"),
        (
            "abs arena",
            "abs source: Arena[64]",
            "Option[Arena[64]].Some(source)",
            "InvalidOwnerInitializer",
        ),
        ("ins buffer", "ins source: Buffer", "Mine.Buffer(source)", "EscapingLoan"),
    ] {
        let source = format!("{PRELUDE}verb borrow({params}) {{ erg holder = {construct}; }}");
        let (tokens, _) = scan(&source);
        let error = analyze(&parse(tokens).expect("source should parse")).expect_err(label).kind;
        assert!(format!("{error:?}").starts_with(expected), "{label}: {error:?}");
    }
}

#[test]
fn a_payload_still_accepts_fresh_values_and_scalar_sources() {
    for (label, body) in [
        ("fresh arena", "erg holder = Option[Arena[64]].Some(Arena[64]()); print(1);"),
        ("fresh buffer", "erg holder = Mine.Buffer(Buffer[4]); print(1);"),
        (
            "moved arena is not used again",
            "erg source: Arena[64] = Arena[64](); erg holder = Mine.Arena(source); print(1);",
        ),
        (
            "scalar source stays readable",
            "erg count: Int = 1; erg holder = Mine.Count(count); print(count);",
        ),
        (
            "result scalar stays readable",
            "erg count: Int = 1; erg holder = Result[Int, Int].Ok(count); print(count);",
        ),
        (
            "placed value read before the arena moves",
            "erg source: Arena[64] = Arena[64](); erg node = source.place(value: Node { value: 1, }); print(node.value); erg holder = Option[Arena[64]].Some(source);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}
