//! A `pack` follows ADR-0124 like a struct: every operation that stores or
//! passes it moves its source, a read does not, and `meta copy` declares that
//! the written `copy(value: abs x)` may duplicate it.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } \
pack Line { erg storage: Array[u8, 4]; layout little; fields { erg kind: u8 at 0; erg size: u16 at 8; abs _reserved: u8 at 24 = 0; } } \
struct Holder { held: Cell, } \
verb take_erg(erg held: Cell) {} verb take_dat(dat held: Cell) {} ";

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    analyze_source(&format!("{PRELUDE}verb main() {{ {body} }}"))
}

const SOURCE: &str = "erg source = Cell { storage: 1u16, }; ";

#[test]
fn a_pack_moves_through_every_operation() {
    for (label, operation) in [
        ("binding", "erg other = source;"),
        ("assignment", "erg other = Cell { storage: 2u16, }; other = source;"),
        ("field initializer", "erg holder = Holder { held: source, };"),
        (
            "field assignment",
            "erg holder = Holder { held: Cell { storage: 2u16, }, }; holder.held = source;",
        ),
        ("option payload", "erg held = Option[Cell].Some(source);"),
        ("result payload", "erg held = Result[Cell, Int].Ok(source);"),
        ("erg argument", "take_erg(held: source);"),
        ("explicit erg argument", "take_erg(held: erg source);"),
        ("dat argument", "take_dat(held: dat source);"),
        ("element assignment", "erg items: Array[Cell, 2] = Array[Cell, 2](); items[0] = source;"),
    ] {
        let error = analyze_main(&format!("{SOURCE}{operation} print(int: source.kind);"))
            .expect_err(&format!("{label}: the operation must move the pack"));
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "source"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn an_array_backed_pack_moves_like_an_integer_backed_one() {
    let body = "erg source = Line { storage: Array[u8, 4](), }; erg other = source; print(int: source.kind);";
    let error = analyze_main(body).expect_err("the binding must move the array-backed pack");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "source"));
}

#[test]
fn a_returned_pack_moves_its_source() {
    let source = format!(
        "{PRELUDE}verb give(erg held: Cell) -> Cell {{ return held; print(int: held.kind); }} verb main() {{}}"
    );
    let error = analyze_source(&source).expect_err("the return must move the pack");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "held"));
}

#[test]
fn reads_do_not_move_a_pack() {
    let body = "erg source = Cell { storage: 1u16, }; \
        print(int: source.kind); print(int: source.kind + source.size); \
        erg same: Bool = source.kind == source.size; print(int: source.kind);";
    analyze_main(body).expect("field reads must leave the source usable");
    let view = "verb show(abs held: Cell) -> u8 { return held.kind; } \
        verb main() { erg source = Cell { storage: 1u16, }; erg first = show(held: abs source); \
        erg second = show(held: abs source); print(int: source.kind + first + second); }";
    analyze_source(&format!("{PRELUDE}{view}")).expect("an abs argument must not move");
}

#[test]
fn a_meta_copy_pack_is_still_moved_and_copied_only_by_the_written_copy() {
    let declarations = "meta copy pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } ";
    let moved = analyze_source(&format!(
        "{declarations}verb main() {{ erg cell = Cell {{ storage: 1u16, }}; erg other = cell; print(int: cell.kind); }}"
    ))
    .expect_err("meta copy does not make a copy implicit");
    assert!(matches!(&moved, SemanticErrorKind::UseAfterMove { name } if name == "cell"));
    analyze_source(&format!(
        "{declarations}verb main() {{ erg cell = Cell {{ storage: 1u16, }}; erg other = copy(value: abs cell); print(int: cell.kind + other.kind); }}"
    ))
    .expect("the written copy leaves the source usable");
}

#[test]
fn a_written_copy_of_a_pack_is_accepted_wherever_a_value_is_stored() {
    let declarations = "meta copy pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } } \
        struct Holder { held: Cell, } verb take(erg held: Cell) {} ";
    let stored = "erg source = Cell { storage: 1u16, }; \
        erg holder = Holder { held: copy(value: abs source), }; \
        holder.held = copy(value: abs source); \
        erg items: Array[Cell, 2] = Array[Cell, 2](); items[0] = copy(value: abs source); \
        erg option = Option[Cell].Some(copy(value: abs source)); \
        erg lent = copy(value: abs source); take(held: lent); print(int: source.kind);";
    analyze_source(&format!("{declarations}verb main() {{ {stored} }}"))
        .expect("a written copy leaves the source usable in every position");
}

#[test]
fn meta_copy_is_accepted_on_every_kind_of_pack() {
    for (label, source) in [
        (
            "integer storage",
            "meta copy pack Cell { erg storage: u16; layout little; fields { erg kind: u8 at 0; erg size: u8 at 8; } }",
        ),
        (
            "byte array storage",
            "meta copy pack Line { erg storage: Array[u8, 4]; layout big; fields { erg word: u32 at 0; } }",
        ),
        (
            "open pack",
            "meta copy open pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } }",
        ),
        (
            "array field",
            "meta copy pack Links { erg storage: Array[u8, 8]; layout little; fields { erg links: Array[u32, 2] at 0; } }",
        ),
        (
            "inside a meta copy struct",
            "meta copy pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } } meta copy struct Frame { cell: Cell, cells: Array[Cell, 2], }",
        ),
    ] {
        analyze_source(&format!("{source} verb main() {{}}"))
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn meta_copy_rejects_a_struct_that_holds_a_pack_that_is_not_a_copy_type() {
    let source = "pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } } \
        meta copy struct Frame { cell: Cell, } verb main() {}";
    let error = analyze_source(source).expect_err("a plain pack is not a copy type");
    let SemanticErrorKind::InvalidCopyType { declaration, reason } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(declaration, "Frame");
    assert!(reason.contains("`cell`"), "{reason}");
}

#[test]
fn meta_copy_rejects_a_pack_with_a_drop_contract() {
    let source = "meta copy pack Cell { erg storage: u8; layout little; fields { erg kind: u8 at 0; } } \
        perform Drop for Cell { verb drop(ins self: Cell) { } } verb main() {}";
    let error = analyze_source(source).expect_err("a drop contract and a copy cannot coexist");
    let SemanticErrorKind::InvalidCopyType { declaration, reason } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(declaration, "Cell");
    assert!(reason.contains("drop contract"), "{reason}");
}

#[test]
fn copy_rejects_a_pack_that_is_not_a_copy_type() {
    for (label, body) in [
        ("integer storage", "erg v = Cell { storage: 1u16, }; erg c = copy(value: abs v);"),
        (
            "byte array storage",
            "erg v = Line { storage: Array[u8, 4](), }; erg c = copy(value: abs v);",
        ),
    ] {
        let error = analyze_main(body)
            .expect_err(&format!("{label}: copy must reject a pack that is not meta copy"));
        assert!(
            matches!(&error, SemanticErrorKind::InvalidIntrinsicArgument { .. }),
            "{label}: {error:?}"
        );
    }
}
