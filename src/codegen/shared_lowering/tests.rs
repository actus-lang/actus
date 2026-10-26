use super::{
    BackendLayout, BackendLayoutProvider, ProgramLayouts,
    lower_program_to_backend_module_with_layouts,
};
use crate::ast::TypeName;
use crate::codegen::{
    BackendCleanupAction, BackendInstruction, BackendPassingMode, BackendReturnMode,
    BackendValueType,
};
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
fn derives_array_layout_from_pack_storage_in_the_neutral_contract() {
    let source = "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { return 0; }";
    let (tokens, errors) = crate::lexer::scan(source);
    assert!(errors.is_empty(), "pack layout fixture should lex: {errors:?}");
    let program = crate::parser::parse(tokens).expect("pack layout fixture should parse");
    let layouts = ProgramLayouts::new(&program, "test");
    let array = crate::semantic::parse_type_name_key(
        "Array[Register, 2]",
        crate::lexer::SourceSpan::new(0, 0),
    )
    .expect("pack array type should parse");
    assert_eq!(layouts.layout_for(&array), Some(BackendLayout { size: 8, alignment: 4 }));
}

#[test]
fn exposes_pack_bit_field_metadata_to_the_neutral_contract() {
    let source = "pack Register { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs _reserved: u31 at 1 = 0; } } verb main() -> Int { return 0; }";
    let (tokens, errors) = crate::lexer::scan(source);
    assert!(errors.is_empty(), "pack field fixture should lex: {errors:?}");
    let program = crate::parser::parse(tokens).expect("pack field fixture should parse");
    let layouts = ProgramLayouts::new(&program, "test");
    let register =
        crate::semantic::parse_type_name_key("Register", crate::lexer::SourceSpan::new(0, 0))
            .expect("pack type should parse");
    let field = layouts
        .packed_field_for(&register, "enabled")
        .expect("packed field metadata should be available");
    assert_eq!(field.storage, BackendLayout { size: 4, alignment: 4 });
    assert_eq!(field.bit_offset, 0);
    assert_eq!(field.width, 1);
    assert!(!field.big_endian);
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
                payload_type: Some(BackendValueType::Integer { signed: true, width: 32 }),
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

#[test]
fn lowers_drop_cleanup_with_case_branch_and_loop_control_flow() {
    let source = "struct Resource { erg handle: Int, } perform Drop for Resource { verb drop(ins self: Resource) { return; } } verb produce() -> Result[Resource, Int] { return Result[Resource, Int].Ok(Resource { handle: 1, }); } verb main() -> Int { erg result = produce(); return case dat result { Result.Ok(resource) => { for erg index: u32 in 0u32 .. 2u32 { if index == 1u32 { return 0; } } 1; }, Result.Err(_) => 2, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "Drop CFG fixture should lex: {errors:?}");
    let program = parse(tokens).expect("Drop CFG fixture should parse");
    let module =
        super::super::lower_program_to_backend_module("actus_root", &program, Some("main"), None)
            .expect("branch-aware Drop cleanup should lower without non-dominating SSA values");
    let main = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("main"))
        .expect("main function should be present");
    let terminal_cleanup_counts = main.blocks.iter().filter_map(|block| {
        block.instructions.last().and_then(|instruction| {
            matches!(instruction, BackendInstruction::Return { .. }).then(|| {
                block
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(instruction, BackendInstruction::Cleanup { .. }))
                    .count()
            })
        })
    });
    let counts = terminal_cleanup_counts.collect::<Vec<_>>();
    assert!(counts.contains(&1));
    assert!(counts.iter().all(|count| *count <= 1));
}

#[test]
fn lowers_drop_cleanup_when_case_branch_falls_through() {
    let source = "struct Resource { erg handle: Int, } perform Drop for Resource { verb drop(ins self: Resource) { return; } } verb produce() -> Result[Resource, Int] { return Result[Resource, Int].Ok(Resource { handle: 1, }); } verb main() -> Int { erg result = produce(); case dat result { Result.Ok(resource) => { }, Result.Err(_) => { }, }; return 0; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "fallthrough Drop fixture should lex: {errors:?}");
    let program = parse(tokens).expect("fallthrough Drop fixture should parse");
    let module =
        super::super::lower_program_to_backend_module("actus_root", &program, Some("main"), None)
            .expect("case fallthrough should retain a valid cleanup owner");
    let main = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("main"))
        .expect("main function should be present");
    assert!(!main.blocks.is_empty());
}

#[test]
fn lowers_unit_return_case_calls_as_discarded_calls() {
    let source = "verb consume() { return; } verb dispatch(erg flag: Bool) { return case flag { true => { consume(); }, _ => { }, }; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unit case fixture should lex: {errors:?}");
    let program = parse(tokens).expect("unit case fixture should parse");
    let module = super::super::lower_program_to_backend_module("actus_root", &program, None, None)
        .expect("unit return case should lower through the neutral contract");
    let dispatch = module
        .functions
        .iter()
        .find(|function| function.symbol.as_str().ends_with("dispatch"))
        .expect("dispatch function should be present");
    assert!(
        dispatch.blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(instruction, BackendInstruction::Call { result: None, .. })
        )
    );
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
fn lowers_partial_aggregate_cleanup_with_remaining_field_descriptor() {
    let source = "struct Inner { erg first: Buffer, erg second: Buffer, } struct Outer { erg inner: Inner, } verb consume(dat payload: Buffer) { drop(payload); } verb main() -> Int { erg holder = Outer { inner: Inner { first: Buffer[1], second: Buffer[2], }, }; consume(payload: dat holder.inner.first); return 42; }";
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "partial aggregate fixture should lex: {errors:?}");
    let program = parse(tokens).expect("partial aggregate fixture should parse");
    let module =
        super::super::lower_program_to_backend_module("actus_root", &program, Some("main"), None)
            .expect("partial aggregate cleanup should lower through the shared pipeline");
    let cleanup = module
        .functions
        .iter()
        .flat_map(|function| function.blocks.iter().flat_map(|block| block.instructions.iter()))
        .find_map(|instruction| match instruction {
            BackendInstruction::Cleanup {
                action: BackendCleanupAction::DropBindingFields { .. },
                operand: Some(operand),
                ..
            } => Some(operand),
            _ => None,
        })
        .expect("remaining aggregate field should have a neutral cleanup descriptor");
    assert_eq!(cleanup.bridge, super::BackendCleanupBridge::BufferDrop);
    assert_eq!(cleanup.argument, super::BackendCleanupArgument::FieldAddress);
    assert_eq!(cleanup.field.expect("remaining field metadata").offset, 8);
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
