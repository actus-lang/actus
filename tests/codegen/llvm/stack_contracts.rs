#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use actus::codegen::llvm::ObjectOptimizationLevel;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::process::Command;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn emit_stack_fixture(
    module: &BackendModule,
    name: &str,
) -> (std::path::PathBuf, object::File<'static>) {
    let target = actus::target::TargetSpec::parse("x86_64-unknown-linux-gnu")
        .expect("x86-64 ELF target should parse");
    let bytes =
        actus::codegen::llvm::emit_object_bytes_with_configuration_and_options_and_float_policy(
            module,
            &target,
            true,
            false,
            ObjectEmissionOptions {
                emit_stack_size_section: true,
                function_sections: true,
                optimization_level: ObjectOptimizationLevel::Default,
            },
        )
        .expect("LLVM stack fixture should emit");
    let path = std::env::temp_dir().join(format!("actus-{name}-{}.o", std::process::id()));
    std::fs::write(&path, bytes).expect("stack fixture object should be written");
    let data: &'static [u8] = Box::leak(
        std::fs::read(&path).expect("stack fixture object should be readable").into_boxed_slice(),
    );
    let object = object::File::parse(data).expect("stack fixture object should parse");
    (path, object)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn read_stack_sizes(path: &std::path::Path) -> String {
    let output = Command::new(llvm_tool("llvm-readobj"))
        .args(["--stack-sizes"])
        .arg(path)
        .output()
        .expect("llvm-readobj should be available");
    assert!(output.status.success(), "llvm-readobj failed");
    String::from_utf8(output.stdout).expect("llvm-readobj output should be UTF-8")
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn stack_size(report: &str, symbol: &str) -> u64 {
    let marker = format!("Functions: [{symbol}]");
    let entry_start = report.find(&marker).expect("stack report should contain symbol");
    let size_start = report[entry_start..]
        .find("Size: 0x")
        .map(|offset| entry_start + offset + "Size: 0x".len())
        .expect("stack report should contain size");
    let size_end = report[size_start..].find('\n').unwrap_or(report.len() - size_start);
    u64::from_str_radix(report[size_start..size_start + size_end].trim(), 16)
        .expect("stack size should be hexadecimal")
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn large_return_slot_aggregate_has_a_real_caller_frame() {
    let layout = actus::codegen::BackendLayout { size: 256, alignment: 8 };
    let aggregate = BackendValueType::Aggregate { layout };
    let callee_symbol = SymbolIdentity::new("stack", SymbolKind::Verb, "make_large")
        .expect("callee symbol should be valid");
    let caller_symbol = SymbolIdentity::new("stack", SymbolKind::Verb, "large_caller")
        .expect("caller symbol should be valid");
    let parameter = BackendParameter {
        name: "value".to_owned(),
        role: Role::Erg,
        ty: aggregate,
        passing: actus::codegen::BackendPassingMode::Direct,
    };
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        hosted_entry_return: false,
        parameters: vec![parameter.clone()],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::ReturnSlot { layout },
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        hosted_entry_return: false,
        parameters: vec![parameter],
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: aggregate,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
                    arguments: vec![BackendValueId(0)],
                    result: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(1)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    let module = BackendModule { externals: Vec::new(), functions: vec![callee, caller] };
    let (path, object) = emit_stack_fixture(&module, "large-return-slot");
    assert!(object.section_by_name(".stack_sizes").is_some());
    let report = read_stack_sizes(&path);
    assert!(report.contains("stack__verb_large_5fcaller"), "missing large caller: {report}");
    assert_eq!(
        stack_size(&report, "stack__verb_large_5fcaller"),
        0x298,
        "large caller report must include saved registers and its frame"
    );
    let disassembly = Command::new(llvm_tool("llvm-objdump"))
        .args(["-d"])
        .arg(&path)
        .output()
        .expect("llvm-objdump should be available");
    assert!(disassembly.status.success(), "llvm-objdump failed");
    let disassembly = String::from_utf8(disassembly.stdout).expect("disassembly should be UTF-8");
    assert!(
        disassembly.contains("subq\t$0x268, %rsp"),
        "large caller must reserve its frame and sret slot: {disassembly}"
    );
    if std::env::var_os("ACTUS_KEEP_STACK_FIXTURES").is_none() {
        std::fs::remove_file(path).expect("large return-slot object should be removed");
    } else {
        eprintln!("kept large return-slot object at {}", path.display());
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn eight_argument_module(callee_name: &str, caller_name: &str) -> BackendModule {
    let integer = BackendValueType::Integer { signed: false, width: 64 };
    let callee_symbol = SymbolIdentity::new("stack", SymbolKind::Verb, callee_name)
        .expect("callee symbol should be valid");
    let caller_symbol = SymbolIdentity::new("stack", SymbolKind::Verb, caller_name)
        .expect("caller symbol should be valid");
    let parameters = (0..8)
        .map(|index| BackendParameter {
            name: format!("arg{index}"),
            role: Role::Erg,
            ty: integer,
            passing: actus::codegen::BackendPassingMode::Direct,
        })
        .collect::<Vec<_>>();
    let callee = BackendFunction {
        symbol: callee_symbol.clone(),
        hosted_entry_return: false,
        parameters: parameters.clone(),
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![BackendInstruction::Return {
                value: Some(BackendValueId(0)),
                span: actus::lexer::SourceSpan::new(0, 1),
            }],
        }],
        cleanup: Vec::new(),
    };
    let caller = BackendFunction {
        symbol: caller_symbol,
        hosted_entry_return: false,
        parameters,
        return_value: BackendReturn {
            role: Some(Role::Erg),
            ty: integer,
            passing: BackendReturnMode::Direct,
        },
        entry: BackendBlockId(0),
        blocks: vec![BackendBlock {
            parameters: Vec::new(),
            id: BackendBlockId(0),
            instructions: vec![
                BackendInstruction::Call {
                    callee: callee_symbol,
                    arguments: (0..8).map(BackendValueId).collect(),
                    result: Some(BackendValueId(8)),
                    span: actus::lexer::SourceSpan::new(0, 1),
                },
                BackendInstruction::Return {
                    value: Some(BackendValueId(8)),
                    span: actus::lexer::SourceSpan::new(1, 2),
                },
            ],
        }],
        cleanup: Vec::new(),
    };
    BackendModule { externals: Vec::new(), functions: vec![callee, caller] }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn x86_64_non_leaf_call_with_eight_arguments_has_stack_evidence() {
    let module = eight_argument_module("eight_args", "eight_arg_caller");
    let (path, object) = emit_stack_fixture(&module, "eight-args");
    assert!(object.section_by_name(".stack_sizes").is_some());
    let report = read_stack_sizes(&path);
    assert!(
        report.contains("stack__verb_eight_5farg_5fcaller"),
        "missing non-leaf caller: {report}"
    );
    let disassembly = Command::new(llvm_tool("llvm-objdump"))
        .args(["-d"])
        .arg(&path)
        .output()
        .expect("llvm-objdump should be available");
    assert!(disassembly.status.success(), "llvm-objdump failed");
    let disassembly = String::from_utf8(disassembly.stdout).expect("disassembly should be UTF-8");
    assert_eq!(
        stack_size(&report, "stack__verb_eight_5farg_5fcaller"),
        0x18,
        "disabled call-frame optimization must report the outgoing argument area"
    );
    assert!(
        disassembly.contains("subq\t$0x18, %rsp"),
        "non-leaf frame must match its report: {disassembly}"
    );
    assert!(disassembly.contains("call"), "non-leaf fixture must contain a call: {disassembly}");
    if std::env::var_os("ACTUS_KEEP_STACK_FIXTURES").is_none() {
        std::fs::remove_file(path).expect("eight-argument object should be removed");
    } else {
        eprintln!("kept eight-argument object at {}", path.display());
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn separate_llvm_process_shows_call_frame_optimization_hides_pushes() {
    let source_path = std::env::temp_dir()
        .join(format!("actus-call-frame-without-shim-{}.ll", std::process::id()));
    let path = std::env::temp_dir()
        .join(format!("actus-call-frame-without-shim-{}.o", std::process::id()));
    let source = r#"
target triple = "x86_64-unknown-linux-gnu"

define i64 @optimized_callee(i64 %arg0, i64 %arg1, i64 %arg2, i64 %arg3,
                             i64 %arg4, i64 %arg5, i64 %arg6, i64 %arg7) {
entry:
  ret i64 %arg0
}

define i64 @optimized_caller(i64 %arg0, i64 %arg1, i64 %arg2, i64 %arg3,
                             i64 %arg4, i64 %arg5, i64 %arg6, i64 %arg7) {
entry:
  %result = call i64 @optimized_callee(i64 %arg0, i64 %arg1, i64 %arg2,
                                       i64 %arg3, i64 %arg4, i64 %arg5,
                                       i64 %arg6, i64 %arg7)
  ret i64 %result
}
"#;
    std::fs::write(&source_path, source).expect("raw LLVM fixture should be written");
    let output = Command::new(llvm_tool("llc"))
        .args(["-filetype=obj", "-stack-size-section", "-O2", "-o"])
        .arg(&path)
        .arg(&source_path)
        .output()
        .expect("llc should be available");
    assert!(
        output.status.success(),
        "llc without Actus shim failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = read_stack_sizes(&path);
    let disassembly = Command::new(llvm_tool("llvm-objdump"))
        .args(["-d"])
        .arg(&path)
        .output()
        .expect("llvm-objdump should be available");
    assert!(disassembly.status.success(), "llvm-objdump failed");
    let disassembly = String::from_utf8(disassembly.stdout).expect("disassembly should be UTF-8");
    assert_eq!(stack_size(&report, "optimized_caller"), 0x8);
    assert_eq!(
        disassembly.matches("pushq").count(),
        3,
        "optimized call frame must push the return value and two stack arguments"
    );
    assert!(
        disassembly.contains("pushq\t%rax"),
        "optimized frame must use hidden pushes: {disassembly}"
    );
    assert!(
        !disassembly.contains("subq\t$0x18, %rsp"),
        "optimized frame must differ from the safe lowering: {disassembly}"
    );
    std::fs::remove_file(path).expect("optimized object should be removed");
    std::fs::remove_file(source_path).expect("raw LLVM fixture should be removed");
}
