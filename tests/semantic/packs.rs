use actus::lexer::scan;
use actus::parser::parse;
use actus::semantic::{SemanticErrorKind, analyze};

fn analyze_source(source: &str) -> Result<actus::semantic::SemanticModel, SemanticErrorKind> {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    let program = parse(tokens).expect("source should parse");
    analyze(&program).map_err(|error| error.kind)
}

#[test]
fn accepts_fully_covered_u8_u16_and_u32_packs() {
    for source in [
        "pack Byte { erg storage: u8; layout little; fields { erg low: u4 at 0; abs high: u4 at 4; } }",
        "pack Word { erg storage: u16; layout big; fields { erg low: u8 at 0; abs high: u8 at 8; } }",
        "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs mode: u3 at 1; erg channel: u5 at 4; abs _reserved: u23 at 9 = 0; } }",
    ] {
        analyze_source(source).expect("fully covered pack should validate");
    }
}

#[test]
fn accepts_a_bounded_u8_array_as_multi_word_pack_storage() {
    analyze_source(
        "pack CacheLine { erg storage: Array[u8, 64]; layout little; fields { erg word_0: u128 at 0; erg word_1: u128 at 128; erg word_2: u128 at 256; erg word_3: u128 at 384; } }",
    )
    .expect("a 64-byte array-backed pack should validate its frontend layout");
}

#[test]
fn records_multi_word_pack_layout_identity_and_field_metadata() {
    let model = analyze_source(
        "pack CacheLine { erg storage: Array[u8, 64]; layout little; fields { erg word_0: u128 at 0; erg word_1: u128 at 128; erg word_2: u128 at 256; erg word_3: u128 at 384; } }",
    )
    .expect("multi-word pack metadata should be semantic data");
    let layout = &model.pack_layouts[0];
    assert_eq!(layout.storage, "Array[u8,64]");
    assert_eq!(layout.storage_bytes, 64);
    assert_eq!(layout.storage_bits, 512);
    assert_eq!(layout.alignment_bytes, 1);
    assert_eq!(layout.endianness, "little");
    assert_eq!(layout.fields[2].name, "word_2");
    assert_eq!(layout.fields[2].offset, 256);
    assert!(!layout.fields[2].has_default);
    assert!(layout.identity.contains("Array[u8,64]"));
}

#[test]
fn records_pack_field_defaults_in_the_layout_contract() {
    let model = analyze_source(
        "pack Flags { erg storage: u8; layout little; fields { abs enabled: u1 at 0 = 0; abs _reserved: u7 at 1 = 0; } }",
    )
    .expect("defaulted pack fields should be represented in semantic metadata");
    let fields = &model.pack_layouts[0].fields;
    assert!(fields[0].has_default);
    assert!(fields[1].has_default);
}

#[test]
fn accepts_indexed_pack_storage_reads_writes_and_nested_control_flow() {
    analyze_source(
        "pack CacheLine { erg storage: Array[u8, 4]; layout little; fields { erg byte_0: u8 at 0; erg byte_1: u8 at 8; erg byte_2: u8 at 16; erg byte_3: u8 at 24; } } verb mutate(ins slot: u8) { slot += 1; } verb update(erg column: CacheLine, abs index: Int) { if true { column.storage[index] = 7; column.storage[index] += 1; mutate(slot: ins column.storage[index]); } } verb read(abs column: CacheLine, abs index: Int) -> u8 { return column.storage[index]; }",
    )
    .expect("pack storage indexing should follow ordinary array place rules");
}

#[test]
fn accepts_pack_types_as_bounded_array_elements() {
    analyze_source(
        "pack Cell { erg storage: Array[u8, 2]; layout little; fields { erg marker: u8 at 0; erg tail: u8 at 8; } } struct Fabric { erg cells: Array[Cell, 64], }",
    )
    .expect("declared packs should be valid array element types");
}

#[test]
fn rejects_unknown_pack_types_as_array_elements() {
    let error = analyze_source("struct Fabric { erg cells: Array[MissingPack, 2], }")
        .expect_err("unknown array pack elements must be rejected");
    assert!(matches!(error, SemanticErrorKind::UnknownType { name } if name == "MissingPack"));
}

#[test]
fn rejects_a_constant_index_outside_pack_storage_capacity() {
    let error = analyze_source(
        "pack CacheLine { erg storage: Array[u8, 2]; layout little; fields { erg byte_0: u8 at 0; erg byte_1: u8 at 8; } } verb read(abs column: CacheLine) -> u8 { return column.storage[2]; }",
    )
    .expect_err("pack storage indexing must use its declared byte capacity");
    assert!(matches!(
        error,
        SemanticErrorKind::IndexOutOfBounds { index, capacity } if index == "2" && capacity == "2"
    ));
}

#[test]
fn rejects_indexed_mutation_through_an_abs_pack_storage_owner() {
    let error = analyze_source(
        "pack CacheLine { erg storage: Array[u8, 2]; layout little; fields { erg byte_0: u8 at 0; erg byte_1: u8 at 8; } } verb update(abs column: CacheLine) { column.storage[0] = 7; }",
    )
    .expect_err("an abs pack must not expose mutable indexed storage");
    assert!(matches!(
        error,
        SemanticErrorKind::InvalidMutation { name } if name == "column"
    ));
}

#[test]
fn rejects_invalid_multi_word_pack_storage_contracts() {
    for source in [
        "pack WrongElement { erg storage: Array[u16, 64]; layout little; fields { erg word_0: u128 at 0; erg word_1: u128 at 128; erg word_2: u128 at 256; erg word_3: u128 at 384; } }",
        "pack Empty { erg storage: Array[u8, 0]; layout little; fields { } }",
        "struct Config { erg capacity: Usize, } pack Runtime { erg storage: Array[u8, N]; layout little; fields { } }",
    ] {
        assert!(matches!(
            analyze_source(source),
            Err(SemanticErrorKind::InvalidPackStorage { .. })
        ));
    }
}

#[test]
fn rejects_unsupported_pack_storage_types() {
    for source in [
        "pack Signed { erg storage: i32; layout little; fields { erg all: u32 at 0; } }",
        "pack Float { erg storage: f32; layout little; fields { erg all: u32 at 0; } }",
    ] {
        assert!(matches!(
            analyze_source(source),
            Err(SemanticErrorKind::InvalidPackStorage { .. })
        ));
    }
}

#[test]
fn rejects_duplicate_pack_names() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg all: u8 at 0; } } pack Control { erg storage: u8; layout little; fields { erg all: u8 at 0; } }",
    )
    .expect_err("duplicate pack names must fail");
    assert!(matches!(
        error,
        SemanticErrorKind::DuplicatePackName { name } if name == "Control"
    ));
}

#[test]
fn rejects_pack_field_overlaps_with_bit_details() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u4 at 0; abs second: u4 at 3; } }",
    )
    .expect_err("overlapping fields must be rejected");
    assert!(matches!(
        error,
        SemanticErrorKind::PackFieldOverlap { field, other, start: 3, end: 4, .. }
            if field == "second" && other == "first"
    ));
}

#[test]
fn rejects_pack_fields_beyond_storage_capacity() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u4 at 0; abs second: u5 at 4; } }",
    )
    .expect_err("out-of-bounds fields must be rejected");
    assert!(matches!(
        error,
        SemanticErrorKind::PackFieldOutOfBounds { field, offset: 4, width: 5, capacity: 8, .. }
            if field == "second"
    ));
}

#[test]
fn rejects_uncovered_bits_without_reserved_field() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg first: u3 at 0; abs second: u4 at 4; } }",
    )
    .expect_err("gaps must require an explicit reserved field");
    assert!(matches!(error, SemanticErrorKind::PackUncoveredBits { start: 3, end: 4, .. }));
}

#[test]
fn accepts_pack_literals_and_erg_field_assignment() {
    analyze_source(
        "pack Control { erg storage: u8; layout little; fields { erg enabled: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg control = Control { storage: 0, }; control.enabled = 1; return control.enabled; }",
    )
    .expect("pack storage and erg fields should be usable");
}

#[test]
fn rejects_writes_to_abs_pack_fields() {
    let error = analyze_source(
        "pack Control { erg storage: u8; layout little; fields { abs mode: u1 at 0; abs _reserved: u7 at 1; } } verb main() -> Int { erg control = Control { storage: 0, }; control.mode = 1; return 0; }",
    )
    .expect_err("abs pack fields must remain read-only");
    assert!(
        matches!(error, SemanticErrorKind::InvalidFieldAssignmentTarget { field } if field == "mode")
    );
}
