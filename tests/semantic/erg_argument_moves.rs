//! An `erg` parameter is a new owner inside the callee, which cleans it up at
//! its scope end (ADR-0016, ADR-0059). A move-only argument therefore leaves
//! the caller exactly as for a `dat` parameter, with or without the explicit
//! `erg` role; a scalar is read and stays usable.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Node { value: Int, } struct Inner { number: Int, } \
struct HoldBuffer { erg held: Buffer, } \
verb hold_buffer(erg held: Buffer) {} verb hold_arena(erg held: Arena[64]) {} \
verb hold_array(erg held: Array[Buffer, 2]) {} \
verb hold_option(erg held: Option[Buffer]) {} verb hold_owner(erg held: HoldBuffer) {} \
verb hold_scalar(erg held: Int) {} verb hold_plain(erg held: Inner) {} verb take_buffer(dat held: Buffer) {} \
verb hold_two(erg first: Buffer, erg second: Buffer) {} ";

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let source = format!("{PRELUDE}verb main() {{ {body} }}");
    let (tokens, errors) = scan(&source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

const ARENA: &str = "erg source: Arena[64] = Arena[64](); ";
const BUFFER: &str = "erg source: Buffer = Buffer[4]; ";
const ARRAY: &str = "erg source: Array[Buffer, 2] = Array[Buffer, 2](); ";
const OPTION: &str = "erg source: Option[Buffer] = Option[Buffer].None; ";
const OWNER: &str = "erg source = HoldBuffer { held: Buffer[4], }; ";

#[test]
fn an_erg_parameter_moves_every_move_only_argument() {
    for (label, declare, call, reuse) in [
        ("buffer", BUFFER, "hold_buffer(held: source);", "append(source, 1u8);"),
        ("buffer drop", BUFFER, "hold_buffer(held: source);", "drop(source);"),
        ("buffer positional", BUFFER, "hold_buffer(source);", "drop(source);"),
        ("buffer explicit role", BUFFER, "hold_buffer(held: erg source);", "drop(source);"),
        (
            "arena",
            ARENA,
            "hold_arena(held: source);",
            "erg node = source.place(value: Node { value: 1, });",
        ),
        ("arena drop", ARENA, "hold_arena(held: source);", "drop(source);"),
        ("array", ARRAY, "hold_array(held: source);", "drop(source);"),
        ("generic enum", OPTION, "hold_option(held: source);", "drop(source);"),
        ("struct with an owned field", OWNER, "hold_owner(held: source);", "drop(source);"),
        ("same call twice", BUFFER, "hold_buffer(held: source);", "hold_buffer(held: source);"),
    ] {
        let error = analyze_main(&format!("{declare}{call} {reuse}"))
            .expect_err(&format!("{label}: the call must consume its argument"));
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name }
                | SemanticErrorKind::DoubleDrop { name } if name == "source"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn an_erg_parameter_agrees_with_a_dat_parameter() {
    let erg = analyze_main(&format!("{BUFFER}hold_buffer(held: source); append(source, 1u8);"));
    let dat = analyze_main(&format!("{BUFFER}take_buffer(held: source); append(source, 1u8);"));
    assert_eq!(erg.expect_err("erg"), dat.expect_err("dat"));
}

#[test]
fn an_erg_parameter_moves_the_erg_field_it_is_given() {
    let error = analyze_main(&format!(
        "{BUFFER}erg owner = HoldBuffer {{ held: source, }}; hold_buffer(held: owner.held); append(owner.held, 1u8);"
    ))
    .expect_err("the source field must be moved");
    assert!(matches!(error, SemanticErrorKind::UseAfterMove { .. }), "{error:?}");
}

#[test]
fn an_erg_parameter_follows_the_arena_release_rules() {
    let placed =
        "erg source: Arena[64] = Arena[64](); erg node = source.place(value: Node { value: 1, }); ";
    let live = analyze_main(&format!(
        "{placed}abs view = ref node; hold_arena(held: source); print(view.value);"
    ))
    .expect_err("an arena cannot move into a call while a view is live");
    assert!(matches!(live, SemanticErrorKind::ArenaReferenceLive { arena, reference }
        if arena == "source" && reference == "view"));
    let released = analyze_main(&format!("{placed}hold_arena(held: source); print(node.value);"))
        .expect_err("a placed value cannot be read once its arena moved into a call");
    assert!(matches!(released, SemanticErrorKind::ArenaReferenceReleased { arena, reference }
        if arena == "source" && reference == "node"));
}

#[test]
fn an_erg_parameter_still_rejects_an_argument_that_is_not_an_active_owner() {
    for (label, params, call) in [
        ("abs view", "abs source: Buffer", "hold_buffer(held: source);"),
        ("ins loan", "ins source: Buffer", "hold_buffer(held: source);"),
    ] {
        let source = format!("{PRELUDE}verb borrow({params}) {{ {call} }}");
        let (tokens, _) = scan(&source);
        let error = analyze(&parse(tokens).expect("source should parse")).expect_err(label).kind;
        assert!(
            matches!(error, SemanticErrorKind::InvalidArgumentRole { .. }),
            "{label}: {error:?}"
        );
    }
    let frozen =
        analyze_main(&format!("{BUFFER}abs view = ref source; hold_buffer(held: source);"))
            .expect_err("a frozen owner cannot be given away");
    assert!(matches!(frozen, SemanticErrorKind::InvalidArgumentRole { .. }), "{frozen:?}");
}

#[test]
fn an_erg_parameter_rejects_the_same_resource_twice_in_one_call() {
    let error = analyze_main(&format!("{BUFFER}hold_two(first: source, second: source);"))
        .expect_err("one buffer cannot become two owners");
    assert!(matches!(error, SemanticErrorKind::UseAfterMove { name } if name == "source"));
}

#[test]
fn an_erg_parameter_still_accepts_scalars_and_unused_owners() {
    for (label, body) in [
        (
            "scalar reused",
            "erg count: Int = 1; hold_scalar(held: count); hold_scalar(held: count); print(count);",
        ),
        (
            "plain data struct keeps value semantics",
            "erg plain = Inner { number: 1, }; hold_plain(held: plain); print(plain.number);",
        ),
        (
            "moved and not read again",
            "erg source: Buffer = Buffer[4]; hold_buffer(held: source); print(1);",
        ),
        ("moved arena", "erg source: Arena[64] = Arena[64](); hold_arena(held: source); print(1);"),
        (
            "dat still moves",
            "erg source: Buffer = Buffer[4]; take_buffer(held: dat source); print(1);",
        ),
    ] {
        analyze_main(body).unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}
