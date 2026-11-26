//! No struct or enum is duplicated implicitly (ADR-0124). Every operation that
//! stores or passes a value moves its source, for plain data as for a
//! resource, a read does not, and `meta copy` declares the types that the
//! written `copy(value: abs x)` may duplicate.
use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

const PRELUDE: &str = "struct Inner { number: Int, } struct Holder { held: Inner, } \
enum Mode { Idle, Busy, } struct Slot { mode: Mode, } \
verb take_erg(erg held: Inner) {} verb take_dat(dat held: Inner) {} \
verb take_mode(erg mode: Mode) {} verb take_mode_dat(dat mode: Mode) {} ";

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

fn analyze_main(body: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    analyze_source(&format!("{PRELUDE}verb main() {{ {body} }}"))
}

const STRUCT: &str = "erg source = Inner { number: 1, }; ";
const ENUM: &str = "erg source = Mode.Idle; ";

#[test]
fn a_plain_struct_moves_through_every_operation() {
    for (label, operation) in [
        ("binding", "erg other = source;"),
        ("assignment", "erg other = Inner { number: 2, }; other = source;"),
        ("field initializer", "erg holder = Holder { held: source, };"),
        (
            "field assignment",
            "erg holder = Holder { held: Inner { number: 2, }, }; holder.held = source;",
        ),
        ("option payload", "erg held = Option[Inner].Some(source);"),
        ("result payload", "erg held = Result[Inner, Int].Ok(source);"),
        ("erg argument", "take_erg(held: source);"),
        ("explicit erg argument", "take_erg(held: erg source);"),
        ("dat argument", "take_dat(held: dat source);"),
        (
            "element assignment",
            "erg items: Array[Inner, 2] = Array[Inner, 2](); items[0] = source;",
        ),
    ] {
        let error = analyze_main(&format!("{STRUCT}{operation} print(source.number);"))
            .expect_err(&format!("{label}: the operation must move the struct"));
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "source"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn a_plain_enum_moves_through_every_operation() {
    for (label, operation) in [
        ("binding", "erg other = source;"),
        ("assignment", "erg other = Mode.Busy; other = source;"),
        ("field initializer", "erg slot = Slot { mode: source, };"),
        ("field assignment", "erg slot = Slot { mode: Mode.Busy, }; slot.mode = source;"),
        ("option payload", "erg held = Option[Mode].Some(source);"),
        ("result payload", "erg held = Result[Mode, Int].Ok(source);"),
        ("erg argument", "take_mode(mode: source);"),
        ("dat argument", "take_mode_dat(mode: dat source);"),
        ("element assignment", "erg items: Array[Mode, 2] = Array[Mode, 2](); items[0] = source;"),
    ] {
        let error =
            analyze_main(&format!("{ENUM}{operation} if source == Mode.Idle {{ print(1); }}"))
                .expect_err(&format!("{label}: the operation must move the enum"));
        assert!(
            matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "source"),
            "{label}: {error:?}"
        );
    }
}

#[test]
fn a_user_enum_payload_moves_a_plain_enum() {
    let source = "enum Mode { Idle, Busy, } enum Wrap { Held(Mode), Empty, } verb main() { erg source = Mode.Idle; erg wrapped = Wrap.Held(source); if source == Mode.Idle { print(1); } }";
    let error = analyze_source(source).expect_err("the payload must move the enum");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "source"));
}

#[test]
fn a_returned_struct_moves_its_source() {
    let source = format!(
        "{PRELUDE}verb give(erg held: Inner) -> Inner {{ return held; print(held.number); }} verb main() {{}}"
    );
    let error = analyze_source(&source).expect_err("the return must move the struct");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "held"));
}

#[test]
fn a_struct_read_from_a_plain_field_cannot_be_moved_out() {
    let body = "erg holder = Holder { held: Inner { number: 1, }, }; erg other = holder.held;";
    let error = analyze_main(body).expect_err("a plain field is not a movable place");
    assert!(matches!(&error, SemanticErrorKind::InvalidDatArgument { .. }), "{error:?}");
}

#[test]
fn reads_do_not_move_a_struct_or_an_enum() {
    let body = "erg source = Inner { number: 1, }; erg mode = Mode.Idle; \
        print(source.number); print(source.number + source.number); \
        erg same = mode == Mode.Idle; erg other: Bool = mode != Mode.Busy; \
        erg value: Int = case abs mode { Mode.Idle => 1, Mode.Busy => 2, }; \
        print(source.number); print(value);";
    analyze_main(body).expect("reads must leave the sources usable");
    let view = "verb show(abs held: Inner) -> Int { return held.number; } \
        verb main() { erg source = Inner { number: 1, }; erg first = show(held: abs source); \
        erg second = show(held: abs source); print(source.number + first + second); }";
    analyze_source(&format!("{PRELUDE}{view}")).expect("an abs argument must not move");
}

#[test]
fn scalars_keep_their_rules() {
    let body = "erg count: Int = 1; take_count(held: count); take_count(held: count); \
        erg other = count; print(count);";
    let source = format!("verb take_count(erg held: Int) {{}} verb main() {{ {body} }}");
    // A scalar binding moves (bug #93 is separate); a scalar argument is a value.
    let error = analyze_source(&source).expect_err("the binding moves a scalar as before");
    assert!(matches!(&error, SemanticErrorKind::UseAfterMove { name } if name == "count"));
    let arguments = "verb take_count(erg held: Int) {} verb main() { erg count: Int = 1; take_count(held: count); take_count(held: count); print(count); }";
    analyze_source(arguments).expect("a scalar passed to an erg parameter stays usable");
}

#[test]
fn a_copy_type_is_still_moved_and_copied_only_by_the_written_copy() {
    let declarations = "meta copy struct Stamp { tick: u32, } meta copy enum Level { Low, High, } ";
    let moved = analyze_source(&format!(
        "{declarations}verb main() {{ erg stamp = Stamp {{ tick: 1u32, }}; erg other = stamp; print(int: stamp.tick); }}"
    ))
    .expect_err("meta copy does not make a copy implicit");
    assert!(matches!(&moved, SemanticErrorKind::UseAfterMove { name } if name == "stamp"));
    let level = analyze_source(&format!(
        "{declarations}verb main() {{ erg level = Level.Low; erg other = level; if level == Level.Low {{ print(1); }} }}"
    ))
    .expect_err("meta copy does not make an enum copy implicit");
    assert!(matches!(&level, SemanticErrorKind::UseAfterMove { name } if name == "level"));
    analyze_source(&format!(
        "{declarations}verb main() {{ erg stamp = Stamp {{ tick: 1u32, }}; erg other = copy(value: abs stamp); erg level = Level.Low; erg again = copy(value: abs level); print(int: stamp.tick + other.tick); if level == Level.Low {{ print(1); }} }}"
    ))
    .expect("the written copy leaves the source usable");
}

#[test]
fn a_written_copy_is_accepted_wherever_a_value_is_stored() {
    let declarations = "meta copy struct Stamp { tick: u32, } struct Holder { stamp: Stamp, } \
        meta copy enum Level { Low, High, } enum Wrap { Held(Level), Empty, } \
        verb take(erg stamp: Stamp) {} ";
    let stored = "erg source = Stamp { tick: 1u32, }; erg level = Level.Low; \
        erg holder = Holder { stamp: copy(value: abs source), }; \
        holder.stamp = copy(value: abs source); \
        erg items: Array[Stamp, 2] = Array[Stamp, 2](); items[0] = copy(value: abs source); \
        erg option = Option[Stamp].Some(copy(value: abs source)); \
        erg wrapped = Wrap.Held(copy(value: abs level)); \
        erg lent = copy(value: abs source); take(stamp: lent); \
        print(int: source.tick); if level == Level.Low { print(1); }";
    analyze_source(&format!("{declarations}verb main() {{ {stored} }}"))
        .expect("a written copy leaves the source usable in every position");
}

#[test]
fn copy_accepts_every_copy_type_and_rejects_the_rest() {
    let declarations = "meta copy struct Stamp { tick: u32, } struct Plain { tick: u32, } \
        enum PlainMode { Idle, } ";
    for (label, body) in [
        ("plain struct", "erg v = Plain { tick: 1u32, }; erg c = copy(value: abs v);"),
        ("plain enum", "erg v = PlainMode.Idle; erg c = copy(value: abs v);"),
        ("buffer", "erg v: Buffer = Buffer[4]; erg c = copy(value: abs v);"),
    ] {
        let error = analyze_source(&format!("{declarations}verb main() {{ {body} }}"))
            .expect_err(&format!("{label}: copy must reject a type that is not a copy type"));
        assert!(
            matches!(&error, SemanticErrorKind::InvalidIntrinsicArgument { .. }),
            "{label}: {error:?}"
        );
    }
    let accepted = "erg v = Stamp { tick: 1u32, }; erg c = copy(value: abs v); \
        erg n: Int = 4; erg m = copy(value: abs n); erg b: u8 = 1u8; erg d = copy(value: abs b);";
    analyze_source(&format!("{declarations}verb main() {{ {accepted} print(int: c.tick); }}"))
        .expect("scalars and a meta copy struct are copy types");
}

#[test]
fn copy_requires_the_abs_role() {
    let source = "meta copy struct Stamp { tick: u32, } verb main() { erg v = Stamp { tick: 1u32, }; erg c = copy(value: v); }";
    let error = analyze_source(source).expect_err("the argument must be an abs view");
    assert!(matches!(&error, SemanticErrorKind::InvalidArgumentRole { .. }), "{error:?}");
}

#[test]
fn meta_copy_accepts_types_made_of_copy_types() {
    for (label, source) in [
        (
            "scalars",
            "meta copy struct Frame { a: Int, b: Usize, c: Bool, d: u8, e: i64, f: f32, g: f64, }",
        ),
        (
            "nested struct",
            "meta copy struct Stamp { tick: u32, } meta copy struct Frame { stamp: Stamp, }",
        ),
        (
            "nested enum",
            "meta copy enum Level { Low, High, } meta copy struct Frame { level: Level, }",
        ),
        ("array of scalars", "meta copy struct Frame { values: Array[u8, 4], }"),
        (
            "array of copy structs",
            "meta copy struct Stamp { tick: u32, } meta copy struct Frame { stamps: Array[Stamp, 2], }",
        ),
        (
            "enum with payloads",
            "meta copy enum Sample { Missing, Reading(u16), Pair { low: u8, high: u8, }, }",
        ),
        (
            "enum with a copy enum",
            "meta copy enum Level { Low, High, } meta copy enum Sample { Missing, At(Array[Level, 2]), Now(Level), }",
        ),
        ("with a representation", "meta repr(C) meta copy struct Frame { a: u8, b: u32, }"),
    ] {
        analyze_source(&format!("{source} verb main() {{}}"))
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    }
}

#[test]
fn meta_copy_rejects_every_field_that_is_not_a_copy_type() {
    let support = "struct Plain { tick: u32, } enum PlainMode { Idle, } \
        struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } ";
    for (label, declaration, field) in [
        ("buffer", "meta copy struct Frame { n: Int, held: Buffer, }", "held"),
        ("arena", "meta copy struct Frame { held: Arena[64], }", "held"),
        ("abs view", "meta copy struct Frame { abs held: Plain, }", "held"),
        ("non-copy struct", "meta copy struct Frame { held: Plain, }", "held"),
        ("non-copy enum", "meta copy struct Frame { held: PlainMode, }", "held"),
        ("drop type", "meta copy struct Frame { held: Counter, }", "held"),
        ("array of non-copy", "meta copy struct Frame { held: Array[Plain, 2], }", "held"),
        ("string", "meta copy struct Frame { held: String, }", "held"),
        ("option of copy", "meta copy struct Frame { held: Option[u8], }", "held"),
        ("enum buffer payload", "meta copy enum Sample { Empty, Held(Buffer), }", "Held.0"),
        (
            "enum named payload",
            "meta copy enum Sample { Empty, Held { held: PlainMode, }, }",
            "Held.held",
        ),
    ] {
        let error = analyze_source(&format!("{support}{declaration} verb main() {{}}"))
            .expect_err(&format!("{label}: the declaration must be rejected"));
        let SemanticErrorKind::InvalidCopyType { declaration, reason } = &error else {
            panic!("{label}: {error:?}");
        };
        assert!(declaration == "Frame" || declaration == "Sample", "{label}: {declaration}");
        assert!(reason.contains(&format!("`{field}`")), "{label}: {reason}");
    }
}

#[test]
fn meta_copy_rejects_a_declaration_with_a_drop_contract() {
    let source = "meta copy struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { } } verb main() {}";
    let error = analyze_source(source).expect_err("a drop contract and a copy cannot coexist");
    let SemanticErrorKind::InvalidCopyType { declaration, reason } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(declaration, "Counter");
    assert!(reason.contains("drop contract"), "{reason}");
}

#[test]
fn the_copy_error_points_at_the_field_that_prevents_it() {
    let source = "meta copy struct Frame { n: Int, held: Buffer, } verb main() {}";
    let (tokens, _) = scan(source);
    let program = parse(tokens).expect("source should parse");
    let error = analyze(&program).expect_err("the declaration must be rejected");
    let start = source.find("held").expect("field in source");
    assert_eq!(error.span.start, start);
}
