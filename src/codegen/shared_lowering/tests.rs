use super::{
    BackendLayout, BackendLayoutProvider, ProgramLayouts,
    lower_program_to_backend_module_with_layouts,
};
use crate::ast::TypeName;
use crate::codegen::{BackendInstruction, BackendPassingMode, BackendReturnMode, BackendValueType};
use crate::lexer::scan;
use crate::parser::parse;

#[test]
fn lowers_result_layouts_for_scalar_and_region_payloads() {
    let source = "verb scalar() -> Result[Int, Error] { return; } verb region() -> Result[Region[u32], Error] { return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Result layout fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Result layout fixture should parse");
    let scalar = super::super::signature::lower_verb_signature(
        "test",
        match &program.declarations[0] {
            crate::ast::TopLevelDecl::Verb(verb) => verb,
            _ => unreachable!(),
        },
        &ProgramLayouts::new(&program, "test"),
    )
    .expect("scalar Result signature should lower");
    let region = super::super::signature::lower_verb_signature(
        "test",
        match &program.declarations[1] {
            crate::ast::TopLevelDecl::Verb(verb) => verb,
            _ => unreachable!(),
        },
        &ProgramLayouts::new(&program, "test"),
    )
    .expect("Region Result signature should lower");

    assert!(matches!(
        scalar.return_value.ty,
        crate::codegen::BackendValueType::Enum {
            layout: crate::codegen::BackendEnumLayout {
                layout: BackendLayout { size: 8, alignment: 4 },
                discriminant_offset: 0,
                discriminant_size: 4,
                payload_offset: 4,
                ..
            }
        }
    ));
    assert!(matches!(
        scalar.return_value.passing,
        crate::codegen::BackendReturnMode::ReturnSlot {
            layout: BackendLayout { size: 8, alignment: 4 }
        }
    ));
    assert!(matches!(
        region.return_value.ty,
        crate::codegen::BackendValueType::Enum {
            layout: crate::codegen::BackendEnumLayout {
                layout: BackendLayout { size: 16, alignment: 8 },
                payload_offset: 8,
                payload: Some(BackendLayout { size: 8, alignment: 8 }),
                ..
            }
        }
    ));
}

#[test]
fn lowers_result_constructor_to_neutral_enum_instruction() {
    let source = "verb main() -> Result[Int, Int] { return Result[Int, Int].Ok(7); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Result constructor fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Result constructor fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("Result constructor should lower through the neutral contract");
    assert!(matches!(
        module.functions[0].blocks[0].instructions.as_slice(),
        [
            BackendInstruction::Constant { .. },
            BackendInstruction::EnumConstruct {
                discriminant: 0,
                payload_type: BackendValueType::Integer { signed: true, width: 32 },
                ..
            },
            BackendInstruction::Return { value: Some(_), .. }
        ]
    ));
}

#[test]
fn lowers_plain_result_case_into_neutral_branches() {
    let source = "verb produce() -> Result[Int, Int] { return Result[Int, Int].Ok(7); } verb main() -> Int { erg result = produce(); return case result { Result.Ok(_) => 0, Result.Err(_) => 1, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Result case fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Result case fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("plain Result case should lower through the neutral contract");
    let main = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("main"))
        .expect("main function should be present");
    assert!(main.blocks.iter().any(|block| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BackendInstruction::EnumDiscriminant { .. }))
    }));
    assert!(main.blocks.iter().any(|block| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BackendInstruction::Branch { .. }))
    }));
}

struct RegionLayouts;

impl BackendLayoutProvider for RegionLayouts {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        (type_name.name == "Region").then_some(BackendLayout { size: 32, alignment: 8 })
    }
}

#[test]
fn lowers_region_signature_and_cleanup_through_shared_pipeline() {
    let source = "verb inspect(erg region: Region[u32]) { return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("Region fixture should parse");
    let module = lower_program_to_backend_module_with_layouts(
        "actus_root",
        &program,
        None,
        None,
        &RegionLayouts,
    )
    .expect("Region signature should lower through the shared pipeline");
    let function = &module.functions[0];
    assert!(matches!(function.parameters[0].ty, BackendValueType::Region { .. }));
    assert!(matches!(function.parameters[0].passing, BackendPassingMode::Indirect { .. }));
    assert!(matches!(function.return_value.passing, BackendReturnMode::Direct));
    assert!(
        function
            .cleanup
            .iter()
            .flat_map(|plan| plan.resolved_bridges.iter().flatten())
            .any(|descriptor| descriptor.descriptor.bridge
                == super::BackendCleanupBridge::RegionDrop)
    );
    assert!(matches!(
        function.blocks[0].instructions.as_slice(),
        [BackendInstruction::Cleanup { .. }, BackendInstruction::Return { value: None, .. }]
    ));
}

#[test]
fn lowers_region_call_and_return_through_shared_pipeline() {
    let source = "verb forward(dat region: Region[u32]) -> Region[u32] { return region; } verb main(dat region: Region[u32]) -> Region[u32] { return forward(region: dat region); }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("Region call fixture should parse");
    let module = lower_program_to_backend_module_with_layouts(
        "actus_root",
        &program,
        None,
        None,
        &RegionLayouts,
    )
    .expect("Region call should lower through the shared pipeline");
    let main = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("main"))
        .expect("main function should be present");
    assert!(main.blocks.iter().flat_map(|block| block.instructions.iter()).any(
        |instruction| matches!(
            instruction,
            crate::codegen::BackendInstruction::Call { result: Some(_), .. }
        )
    ));
    assert!(
        main.blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .all(|instruction| !matches!(instruction, BackendInstruction::Cleanup { .. }))
    );
}

#[test]
fn lowers_buffer_cleanup_before_shared_return() {
    let source = "verb inspect(erg buffer: Buffer) { return; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("Buffer fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("Buffer cleanup should lower through the shared pipeline");
    assert!(matches!(
        module.functions[0].blocks[0].instructions.as_slice(),
        [BackendInstruction::Cleanup { .. }, BackendInstruction::Return { value: None, .. }]
    ));
    assert!(
        module.functions[0]
            .cleanup
            .iter()
            .flat_map(|plan| { plan.resolved_bridges.iter().flatten() })
            .any(|descriptor| {
                descriptor.descriptor.bridge == super::BackendCleanupBridge::BufferDrop
            })
    );
}

#[test]
fn lowers_compile_time_integer_casts_as_typed_constants() {
    let source = "verb main() -> u32 { return 4 as u32; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("cast fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("compile-time cast should lower through the shared pipeline");
    assert!(matches!(
        module.functions[0].blocks[0].instructions.as_slice(),
        [
            BackendInstruction::Constant {
                ty: BackendValueType::Integer { signed: false, width: 32 },
                ..
            },
            BackendInstruction::Return { value: Some(_), .. }
        ]
    ));
}

#[test]
fn lowers_runtime_integer_casts_with_checked_neutral_instruction() {
    let source = "verb identity(erg value: Int) -> u32 { return value as u32; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "lexer errors: {errors:?}");
    let program = parse(tokens).expect("runtime cast fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("runtime cast should lower through the shared contract");
    assert!(matches!(
        module.functions[0].blocks[0].instructions.as_slice(),
        [
            BackendInstruction::IntegerCast {
                source_type: BackendValueType::Integer { signed: true, width: 32 },
                target_type: BackendValueType::Integer { signed: false, width: 32 },
                ..
            },
            BackendInstruction::Return { value: Some(_), .. }
        ]
    ));
}
