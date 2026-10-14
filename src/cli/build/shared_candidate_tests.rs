use super::supports_scalar_shared_lowering;
use crate::lexer::scan;
use crate::parser::parse;

#[test]
fn accepts_the_supported_scalar_contract_slice() {
    let source = "verb main() -> Int { return 40 + 2; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("scalar fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_scalar_assignment_in_the_function_scope() {
    let source = "verb main() -> Int { erg value = 40; value += 2; return value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("assignment fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_an_if_statement_without_an_else_in_the_shared_contract() {
    let source = "verb main() -> Int { erg value: u32 = 0u32; if true { value = 41u32; } return value as Int + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("if-without-else fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_checked_scalar_arithmetic_in_the_shared_contract() {
    let source =
        "verb main() -> Int { erg value: u8 = 1u8; erg count: u8 = 8u8; return value << count; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("checked arithmetic fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_scalar_result_construction_and_case_return() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb main() -> Int { erg result = produce(); return case dat result { Result.Ok(value) => value + 1, Result.Err(_) => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("Result fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_nested_option_case_expression_in_arithmetic() {
    let source = "struct Node { value: Int, abs next: Option[abs Node], } verb sum(abs node: Node) -> Int { abs next = ref node.next; return node.value + case next { Option.Some(_) => 1, Option.None => 0, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("nested Option case fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_bounded_buffer_and_region_signatures() {
    let source = "verb create() -> Buffer { return Buffer[16]; } verb inspect(abs region: Region[u8]) { return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("Buffer and Region fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_generic_owned_buffer_field_transfer_in_the_shared_contract() {
    let source = "struct Box[T] { erg item: T, } verb consume(dat value: Buffer) { drop(value); return; } verb main() -> Int { erg boxed: Box[Buffer] = Box[Buffer] { item: Buffer[1], }; consume(value: dat boxed.item); return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic owned transfer fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_nested_generic_owned_buffer_field_transfer_in_the_shared_contract() {
    let source = "struct Box[T] { erg item: T, } struct Pair[T] { erg item: T, } verb consume(dat value: Buffer) { drop(value); return; } verb main() -> Int { erg pair: Pair[Box[Buffer]] = Pair[Box[Buffer]] { item: Box[Buffer] { item: Buffer[1], }, }; consume(value: dat pair.item.item); return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("nested generic owned transfer fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_fixed_array_iteration_in_the_shared_contract() {
    let source = "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); erg total: u32 = 0u32; for erg index: u32 in values { total += values[index]; } return total as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixed-array fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_direct_fixed_array_index_reads_in_the_shared_contract() {
    let source = "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return values[2u32] as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("direct array index fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_fixed_array_element_stores_in_the_shared_contract() {
    let source = "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] = 41u32; return values[2u32] as Int + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixed-array store fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_aggregate_array_element_stores_in_the_shared_contract() {
    let source = "struct Point { x: u32, } verb main() -> Int { erg values: Array[Point, 2] = Array[Point, 2](); values[1u32] = Point { x: 41u32, }; return values[1u32].x as Int + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("aggregate array store fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_nested_aggregate_array_element_stores_in_the_shared_contract() {
    let source = "struct Point { x: u32, } struct Line { start: Point, } verb main() -> Int { erg values: Array[Line, 2] = Array[Line, 2](); values[1u32] = Line { start: Point { x: 41u32, }, }; return values[1u32].start.x as Int + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("nested aggregate array store fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_generic_aggregate_array_element_stores_in_the_shared_contract() {
    let source = "struct Box[T] { item: T, } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; return boxes[1u32].item + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic aggregate array store fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_generic_aggregate_array_element_compound_stores_in_the_shared_contract() {
    let source = "struct Box[T] { erg item: T, } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: 41, }; boxes[1u32].item += 1; return boxes[1u32].item; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic aggregate compound fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_nested_generic_aggregate_array_element_stores_in_the_shared_contract() {
    let source = "struct Box[T] { item: T, } struct Pair[T] { item: T, } verb main() -> Int { erg pairs: Array[Pair[Box[Int]], 2] = Array[Pair[Box[Int]], 2](); pairs[1u32] = Pair[Box[Int]] { item: Box[Int] { item: 41, }, }; return pairs[1u32].item.item + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("nested generic aggregate fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_generic_aggregate_option_view_fields_in_the_shared_contract() {
    let source = "struct Box[T] { abs item: Option[abs T], } verb main() -> Int { erg boxes: Array[Box[Int], 2] = Array[Box[Int], 2](); boxes[1u32] = Box[Int] { item: Option[abs Int].None, }; return case boxes[1u32].item { Option.Some(_) => 0, Option.None => 42, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("generic ownership aggregate fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_fixed_array_element_compound_stores_in_the_shared_contract() {
    let source = "verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[2u32] += 41u32; return values[2u32] as Int + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixed-array compound store fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_dynamic_fixed_array_indices_in_the_shared_contract() {
    let source = "verb read(abs index: u32) -> Int { erg values: Array[u32, 4] = Array[u32, 4](); values[index] += 41u32; return values[index] as Int + 1; } verb main() -> Int { return read(index: abs 2u32); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("dynamic fixed-array fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_fixed_array_parameter_reads_in_the_shared_contract() {
    let source = "verb read(abs values: Array[u32, 4]) -> Int { return values[2u32] as Int; } verb main() -> Int { erg values: Array[u32, 4] = Array[u32, 4](); return read(values: abs values); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("fixed-array parameter fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_scalar_struct_parameter_reads_in_the_shared_contract() {
    let source = "struct Counter { value: Int, } verb read(abs counter: Counter) -> Int { return counter.value; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; return read(counter: abs counter) + 1; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("scalar-struct parameter fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_scalar_struct_parameter_mutation_in_the_shared_contract() {
    let source = "struct Counter { value: Int, } verb increment(ins counter: Counter) { counter.value += 1; return; } verb main() -> Int { erg counter: Counter = Counter { value: 41, }; increment(counter: ins counter); return counter.value; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("scalar-struct mutation fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_block_bodied_result_cases_in_the_shared_contract() {
    let source = "enum Failure { Bad, } verb produce() -> Result[Int, Failure] { return Result[Int, Failure].Ok(41); } verb main() -> Int { erg result = produce(); return case dat result { Result.Err(_) => { return 0; }, Result.Ok(value) => { return value + 1; }, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("block-bodied Result fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}

#[test]
fn accepts_bounded_range_iteration_in_the_shared_contract() {
    let source = "verb main() -> Int { erg total: u32 = 0u32; for erg index: u32 in 0u32 .. 4u32 { total += index; } return total as Int; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty());
    let program = parse(tokens).expect("range fixture should parse");
    assert!(supports_scalar_shared_lowering(&program));
}
