#![cfg(all(unix, feature = "cranelift", feature = "llvm"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{Architecture, BinaryFormat, Object, ObjectSection, ObjectSymbol, RelocationTarget};

const BACKENDS: [&str; 2] = ["cranelift", "llvm"];
const SOURCE: &str = "import { open std::region };\n\"\"\"Entry point that stores 41 in the first element of a u8 array and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u8, 4] = Array[u8, 4](); values[0] = 41u8; return values[0] as Int + 1; }\n\"\"\"\ncontract:\npurpose:\n    Open a one-element u8 region over the given backing buffer and drop it.\ninputs:\n    backing: owned Buffer moved into the region.\noutputs:\n    0 when the region opened, 1 when it did not.\nownership:\n    The backing buffer is moved into the region and the region is dropped.\nerrors:\n    A RegionError from region_open is mapped to the exit code 1.\nside_effects:\n    Allocates region storage.\n\"\"\"\nmeta effect(\"allocation\") verb boot(erg backing: Buffer) -> Int { erg length: u64 = 1u64; erg start: u64 = 0u64; erg count: u64 = 1u64; erg opened = region_open[u8](backing: dat backing, logical_length: erg length, window_start: erg start, window_count: erg count); return case dat opened { Result.Err(_) => 1, Result.Ok(region) => { drop(region); return 0; }, }; }\n";

fn fixture_root(name: &str, backend: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("actus-backend-parity-{name}-{backend}-{}", std::process::id()))
}

fn write_fixture(
    root: &Path,
    backend: &str,
    target: Option<&str>,
    entry_contract: Option<&str>,
    source: &str,
) -> PathBuf {
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("create parity fixture source directory");
    let target_line = target.map_or(String::new(), |value| format!("target = \"{value}\"\n"));
    let entry_line =
        entry_contract.map_or(String::new(), |value| format!("entry_contract = \"{value}\"\n"));
    fs::write(
        root.join("Actus.toml"),
        format!(
            "[package]\nname = \"backend-parity\"\nversion = \"0.1.0\"\nsource_root = \"src\"\nentry = \"{}\"\n\n[build]\nruntime = \"std\"\nbackend = \"{backend}\"\n{target_line}{entry_line}verify_no_float_ir = true\n",
            if target.is_some() || entry_contract.is_some() { "boot" } else { "main" },
        ),
    )
    .expect("write parity fixture manifest");
    let input = source_root.join("main.act");
    fs::write(&input, source).expect("write parity fixture source");
    input
}

fn build(root: &Path, input: &Path, output: &Path, emit: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            input.to_str().expect("fixture path should be UTF-8"),
            "--strict",
            "--emit",
            emit,
            "-o",
            output.to_str().expect("output path should be UTF-8"),
        ])
        .current_dir(root)
        .output()
        .expect("Actus parity build should start")
}

fn canonical_symbol_name(format: BinaryFormat, name: &str) -> String {
    if format == BinaryFormat::MachO {
        name.strip_prefix('_').unwrap_or(name).to_owned()
    } else {
        name.to_owned()
    }
}

fn symbols(bytes: &[u8], defined: bool) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let format = file.format();
    file.symbols()
        .filter(|symbol| if defined { symbol.is_definition() } else { symbol.is_undefined() })
        .filter_map(|symbol| symbol.name().ok().map(|name| canonical_symbol_name(format, name)))
        .filter(|name| name == "boot" || name.starts_with("actus_"))
        .collect()
}

fn referenced_undefined_symbols(bytes: &[u8]) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let format = file.format();
    let mut symbols = BTreeSet::new();
    for section in file.sections() {
        for (_, relocation) in section.relocations() {
            let RelocationTarget::Symbol(index) = relocation.target() else { continue };
            let symbol = file.symbol_by_index(index).expect("relocation symbol should exist");
            if symbol.is_undefined()
                && let Ok(name) = symbol.name()
                && let name = canonical_symbol_name(format, name)
                && name.starts_with("actus_")
            {
                symbols.insert(name);
            }
        }
    }
    symbols
}

fn undefined_symbols(bytes: &[u8]) -> BTreeSet<String> {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let format = file.format();
    file.symbols()
        .filter(|symbol| symbol.is_undefined())
        .filter_map(|symbol| symbol.name().ok().map(|name| canonical_symbol_name(format, name)))
        .collect()
}

fn external_relocation_contract(bytes: &[u8], expected: &str) -> (Architecture, Vec<u8>) {
    let file = object::File::parse(bytes).expect("parity object should parse");
    let architecture = file.architecture();
    let format = file.format();
    let mut sizes = Vec::new();
    for section in file.sections() {
        for (_, relocation) in section.relocations() {
            let RelocationTarget::Symbol(index) = relocation.target() else { continue };
            let symbol = file.symbol_by_index(index).expect("relocation symbol should exist");
            if symbol.name().ok().map(|name| canonical_symbol_name(format, name)).as_deref()
                == Some(expected)
            {
                sizes.push(relocation.size());
            }
        }
    }
    sizes.sort_unstable();
    (architecture, sizes)
}

fn assert_external_relocation_contract(bytes: &[u8], expected: &str) {
    let (architecture, sizes) = external_relocation_contract(bytes, expected);
    assert!(!sizes.is_empty(), "external relocation is missing for {expected}");
    // Backends may encode the same external call differently on one target:
    // AArch64 LLVM commonly uses CALL26 while Cranelift may materialize the
    // address through two 32-bit relocations. The ABI contract is therefore
    // symbol reachability plus a valid target relocation encoding, not byte-
    // identical relocation records.
    let valid_sizes = match architecture {
        Architecture::Aarch64 => &[26u8, 32, 64][..],
        Architecture::X86_64 => &[32u8, 64][..],
        _ => &[8u8, 16, 26, 32, 64][..],
    };
    assert!(
        sizes.iter().all(|size| valid_sizes.contains(size)),
        "unsupported {architecture:?} relocation width for {expected}: {sizes:?}"
    );
}

#[test]
fn hosted_cli_backends_preserve_exit_status_and_buffer_ownership() {
    let mut statuses = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("hosted", backend);
        let input = write_fixture(&root, backend, None, None, SOURCE);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} hosted build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run hosted parity fixture");
        statuses.push(status.code());
        assert_eq!(status.code(), Some(42), "{backend} hosted runtime result");
        fs::remove_dir_all(root).expect("remove hosted parity fixture");
    }
    assert_eq!(statuses, vec![Some(42), Some(42)]);
}

#[test]
fn hosted_cli_backends_emit_deterministic_objects() {
    for backend in BACKENDS {
        let root = fixture_root("deterministic-object", backend);
        let input = write_fixture(&root, backend, None, None, SOURCE);
        let first = root.join("first.o");
        let second = root.join("second.o");
        let first_build = build(&root, &input, &first, "obj");
        let second_build = build(&root, &input, &second, "obj");
        assert!(
            first_build.status.success() && second_build.status.success(),
            "{backend} deterministic object build failed: {} {}",
            String::from_utf8_lossy(&first_build.stderr),
            String::from_utf8_lossy(&second_build.stderr)
        );
        assert_eq!(
            fs::read(&first).expect("read first deterministic object"),
            fs::read(&second).expect("read second deterministic object"),
            "{backend} object emission is not deterministic"
        );
        fs::remove_dir_all(root).expect("remove deterministic object fixture");
    }
}

#[test]
fn hosted_cli_backends_preserve_external_aggregate_provider_symbols() {
    let source = "meta repr(C) struct Point { x: i32, y: i32, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol inspect, which takes a Point by value and returns an Int.\ninputs:\n    point: owned Point passed by value.\noutputs:\n    The Int returned by the host provider.\nownership:\n    The Point is moved to the provider.\nabi:\n    unsafe extern C symbol inspect taking a repr(C) Point by value.\n\"\"\"\nunsafe extern \"C\" verb inspect(erg point: Point) -> Int;\n\"\"\"Entry point that builds a Point with x 40 and y 2 and returns the result of passing it by value to the foreign inspect verb.\"\"\"\nverb main() -> Int { erg point: Point = Point { x: 40i32, y: 2i32, }; return inspect(point: erg point); }\n";
    let mut providers = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("external-aggregate", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(
            build.status.success(),
            "{backend} external aggregate build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let bytes = fs::read(&output).expect("read external aggregate parity object");
        providers.push(
            undefined_symbols(&bytes)
                .into_iter()
                .filter(|symbol| symbol == "inspect")
                .collect::<BTreeSet<_>>(),
        );
        assert_external_relocation_contract(&bytes, "inspect");
        fs::remove_dir_all(root).expect("remove external aggregate parity fixture");
    }
    assert!(
        providers[0].contains("inspect"),
        "external provider symbol is missing: {:?}",
        providers[0]
    );
    assert_eq!(providers[0], providers[1], "external aggregate provider symbols diverged");
}

#[test]
fn hosted_cli_backends_preserve_checked_representation_provider_contracts() {
    let fixtures = [
        (
            "packed",
            "meta repr(C, packed) struct PackedPoint { prefix: u8, value: u32, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol inspect, which takes a PackedPoint by value and returns an Int.\ninputs:\n    point: owned PackedPoint passed by value.\noutputs:\n    The Int returned by the host provider.\nownership:\n    The PackedPoint is moved to the provider.\nabi:\n    unsafe extern C symbol inspect taking a packed PackedPoint by value.\n\"\"\"\nunsafe extern \"C\" verb inspect(erg point: PackedPoint) -> Int;\n\"\"\"Entry point that builds a PackedPoint with prefix 1 and value 41 and returns the result of passing it by value to the foreign inspect verb.\"\"\"\nverb main() -> Int { erg point: PackedPoint = PackedPoint { prefix: 1u8, value: 41u32, }; return inspect(point: erg point); }\n",
        ),
        (
            "transparent",
            "meta repr(transparent) struct UserId { value: u64, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol inspect, which takes a UserId by value and returns an Int.\ninputs:\n    point: owned UserId passed by value.\noutputs:\n    The Int returned by the host provider.\nownership:\n    The UserId is moved to the provider.\nabi:\n    unsafe extern C symbol inspect taking a UserId wrapper by value.\n\"\"\"\nunsafe extern \"C\" verb inspect(erg point: UserId) -> Int;\n\"\"\"Entry point that builds a UserId holding 42 and returns the result of passing it by value to the foreign inspect verb.\"\"\"\nverb main() -> Int { erg point: UserId = UserId { value: 42u64, }; return inspect(point: erg point); }\n",
        ),
        (
            "aligned",
            "meta repr(C, align(8u32)) struct AlignedValue { value: u8, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol inspect, which takes an AlignedValue by value and returns an Int.\ninputs:\n    point: owned AlignedValue passed by value.\noutputs:\n    The Int returned by the host provider.\nownership:\n    The AlignedValue is moved to the provider.\nabi:\n    unsafe extern C symbol inspect taking an aligned AlignedValue by value.\n\"\"\"\nunsafe extern \"C\" verb inspect(erg point: AlignedValue) -> Int;\n\"\"\"Entry point that builds an AlignedValue holding 42 and returns the result of passing it by value to the foreign inspect verb.\"\"\"\nverb main() -> Int { erg point: AlignedValue = AlignedValue { value: 42u8, }; return inspect(point: erg point); }\n",
        ),
    ];
    for (fixture, source) in fixtures {
        let mut provider_symbols = Vec::new();
        for backend in BACKENDS {
            let root = fixture_root(&format!("representation-{fixture}"), backend);
            let input = write_fixture(&root, backend, None, None, source);
            let output = root.join("main.o");
            let build = build(&root, &input, &output, "obj");
            assert!(
                build.status.success(),
                "{backend} {fixture} representation build failed: {}",
                String::from_utf8_lossy(&build.stderr)
            );
            let bytes = fs::read(&output).expect("read representation parity object");
            provider_symbols.push(
                undefined_symbols(&bytes)
                    .into_iter()
                    .filter(|symbol| symbol == "inspect")
                    .collect::<BTreeSet<_>>(),
            );
            assert_external_relocation_contract(&bytes, "inspect");
            fs::remove_dir_all(root).expect("remove representation parity fixture");
        }
        assert!(
            provider_symbols[0].contains("inspect"),
            "{fixture} provider symbol is missing: {:?}",
            provider_symbols[0]
        );
        assert_eq!(provider_symbols[0], provider_symbols[1], "{fixture} provider symbols diverged");
    }
}

#[test]
fn hosted_cli_backends_preserve_checked_representation_return_contracts() {
    let fixtures = [
        (
            "c",
            "meta repr(C) struct Point { x: i32, y: i32, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol make, which returns a Point by value.\noutputs:\n    A Point built by the host provider.\nownership:\n    The returned Point is owned by the caller.\nabi:\n    unsafe extern C symbol make returning a repr(C) Point.\n\"\"\"\nunsafe extern \"C\" verb make() -> Point;\n\"\"\"Entry point that calls the foreign make verb and returns the x field of the Point it produced, as Int.\"\"\"\nverb main() -> Int { erg point = make(); return point.x as Int; }\n",
        ),
        (
            "packed",
            "meta repr(C, packed) struct PackedPoint { prefix: u8, value: u32, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol make, which returns a PackedPoint by value.\noutputs:\n    A PackedPoint built by the host provider.\nownership:\n    The returned PackedPoint is owned by the caller.\nabi:\n    unsafe extern C symbol make returning a packed PackedPoint.\n\"\"\"\nunsafe extern \"C\" verb make() -> PackedPoint;\n\"\"\"Entry point that calls the foreign make verb and returns the value field of the PackedPoint it produced, as Int.\"\"\"\nverb main() -> Int { erg point = make(); return point.value as Int; }\n",
        ),
        (
            "transparent",
            "meta repr(transparent) struct UserId { value: u64, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol make, which returns a UserId by value.\noutputs:\n    A UserId built by the host provider.\nownership:\n    The returned UserId is owned by the caller.\nabi:\n    unsafe extern C symbol make returning a UserId wrapper.\n\"\"\"\nunsafe extern \"C\" verb make() -> UserId;\n\"\"\"Entry point that calls the foreign make verb and returns the value field of the PackedPoint it produced, as Int.\"\"\"\nverb main() -> Int { erg point = make(); return point.value as Int; }\n",
        ),
        (
            "aligned",
            "meta repr(C, align(8u32)) struct AlignedValue { value: u8, }\n\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol make, which returns an AlignedValue by value.\noutputs:\n    An AlignedValue built by the host provider.\nownership:\n    The returned AlignedValue is owned by the caller.\nabi:\n    unsafe extern C symbol make returning an aligned AlignedValue.\n\"\"\"\nunsafe extern \"C\" verb make() -> AlignedValue;\n\"\"\"Entry point that calls the foreign make verb and returns the value field of the PackedPoint it produced, as Int.\"\"\"\nverb main() -> Int { erg point = make(); return point.value as Int; }\n",
        ),
    ];
    for (fixture, source) in fixtures {
        let mut provider_symbols = Vec::new();
        for backend in BACKENDS {
            let root = fixture_root(&format!("representation-return-{fixture}"), backend);
            let input = write_fixture(&root, backend, None, None, source);
            let output = root.join("main.o");
            let build = build(&root, &input, &output, "obj");
            assert!(
                build.status.success(),
                "{backend} {fixture} representation return build failed: {}",
                String::from_utf8_lossy(&build.stderr)
            );
            let bytes = fs::read(&output).expect("read representation return parity object");
            provider_symbols.push(
                undefined_symbols(&bytes)
                    .into_iter()
                    .filter(|symbol| symbol == "make")
                    .collect::<BTreeSet<_>>(),
            );
            assert_external_relocation_contract(&bytes, "make");
            fs::remove_dir_all(root).expect("remove representation return parity fixture");
        }
        assert!(
            provider_symbols[0].contains("make"),
            "{fixture} return provider symbol is missing: {:?}",
            provider_symbols[0]
        );
        assert_eq!(
            provider_symbols[0], provider_symbols[1],
            "{fixture} return provider symbols diverged"
        );
    }
}

#[test]
fn hosted_cli_backends_preserve_typed_failure_exit_status() {
    let source = "enum Failure { Rejected, }\n\"\"\"\ncontract:\npurpose:\n    Always fail with the rejected failure, giving callers an error result to handle.\noutputs:\n    Result.Err with Failure.Rejected; it never returns Ok.\nerrors:\n    Always returns Failure.Rejected.\n\"\"\"\nverb reject() -> Result[Int, Failure] { return Err(Failure.Rejected); }\n\"\"\"Entry point that calls reject and returns 42 when it fails with an error, or 1 if it unexpectedly succeeds.\"\"\"\nverb main() -> Int { erg result = reject(); return case dat result { Result.Err(_) => 42, Result.Ok(_) => 1, }; }\n";
    let mut statuses = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("typed-failure", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} typed-failure build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        statuses.push(Command::new(&output).status().expect("run typed-failure fixture").code());
        fs::remove_dir_all(root).expect("remove typed-failure parity fixture");
    }
    assert_eq!(statuses, vec![Some(42), Some(42)]);
}

#[test]
fn hosted_cli_backends_reject_the_same_semantic_failure_before_object_emission() {
    let source = "\"\"\"Entry point that calls missing_verb, which is not declared anywhere, so the compiler must report an unknown verb.\"\"\"\nverb main() -> Int { return missing_verb(); }\n";
    for backend in BACKENDS {
        let root = fixture_root("semantic-failure", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(!build.status.success(), "{backend} must reject the unknown call");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(
            diagnostics.contains("E1069"),
            "{backend} changed the semantic diagnostic: {diagnostics}"
        );
        assert!(!output.exists(), "{backend} emitted an object after semantic rejection");
        fs::remove_dir_all(root).expect("remove semantic failure fixture");
    }
}

#[test]
fn hosted_cli_backends_reject_the_same_contract_failures_before_object_emission() {
    let fixtures = [
        (
            "must-use",
            "\"\"\"Returns 0; the must_use attribute asks callers to inspect the status instead of ignoring it.\"\"\"\nmeta must_use(\"check the status\") verb status() -> Int { return 0; }\n\"\"\"Entry point that calls status and discards its result, which the compiler must report as ignoring a must_use result.\"\"\"\nverb main() -> Int { status(); return 0; }\n",
            "E1120",
        ),
        (
            "effect",
            "\"\"\"\ncontract:\npurpose:\n    Return 0 while declaring the filesystem effect, standing in for a load from disk.\noutputs:\n    0.\nside_effects:\n    Declared as a filesystem effect; the body performs none.\n\"\"\"\nmeta effect(\"filesystem\") verb load() -> Int { return 0; }\n\"\"\"Entry point that calls load, which declares a filesystem effect, without declaring that effect itself, so the compiler must report the missing effect.\"\"\"\nverb main() -> Int { load(); return 0; }\n",
            "E1121",
        ),
        (
            "capability",
            "\"\"\"\ncontract:\npurpose:\n    Return 0 while declaring the posix.cloexec capability, standing in for a primitive that needs it.\noutputs:\n    0.\nside_effects:\n    Requires the posix.cloexec capability; the body performs no operation.\n\"\"\"\nmeta capability(\"posix.cloexec\") verb primitive() -> Int { return 0; }\n\"\"\"Entry point that calls primitive, which requires the posix.cloexec capability, without declaring that capability, so the compiler must report the missing capability.\"\"\"\nverb main() -> Int { primitive(); return 0; }\n",
            "E1131",
        ),
        (
            "deterministic",
            "\"\"\"\ncontract:\npurpose:\n    Return 0 while declaring the filesystem effect, standing in for a nondeterministic source.\noutputs:\n    0.\nside_effects:\n    Declared as a filesystem effect; the body performs none.\n\"\"\"\nmeta effect(\"filesystem\") verb source() -> Int { return 0; }\n\"\"\"Verb marked deterministic that calls source, which declares a filesystem effect, so the compiler must reject the call as nondeterministic.\"\"\"\nmeta deterministic verb stable() -> Int { source(); return 0; }\n",
            "E1124",
        ),
        (
            "realtime",
            "\"\"\"Verb marked realtime that contains an unbounded loop which breaks immediately, so the compiler must reject it as not realtime-safe; it returns 0.\"\"\"\nmeta realtime verb tick() -> Int { loop { break; } return 0; }\n",
            "E1126",
        ),
        (
            "bounded",
            "\"\"\"Entry point marked with a loop bound of 3 iterations whose for loop runs from 0 to 4, so the compiler must report the exceeded bound; it returns 0.\"\"\"\nmeta bounded(max_iterations: 3u32) verb main() -> Int { for erg index: u32 in 0u32..4u32 { } return 0; }\n",
            "E1130",
        ),
        ("parser", "verbb main() -> Int { return 0; }\n", "E0009"),
        (
            "unknown-type",
            "\"\"\"Entry point declared with the return type Missing, which does not exist, so the compiler must report an unknown type.\"\"\"\nverb main() -> Missing { return 0; }\n",
            "E1023",
        ),
        (
            "unknown-verb",
            "\"\"\"Entry point that calls the undeclared verb missing and then returns 0, so the compiler must report an unknown verb.\"\"\"\nverb main() -> Int { missing(); return 0; }\n",
            "E1069",
        ),
        (
            "undeclared-identifier",
            "\"\"\"Entry point that returns the undeclared name missing, so the compiler must report an unknown name.\"\"\"\nverb main() -> Int { return missing; }\n",
            "E1003",
        ),
        (
            "use-after-move",
            "\"\"\"\ncontract:\npurpose:\n    Declare the foreign C symbol inspect, which borrows a Buffer and returns no value.\ninputs:\n    view: borrowed Buffer that the provider only reads.\noutputs:\n    No value.\nownership:\n    The buffer stays owned by the caller.\nabi:\n    unsafe extern C symbol inspect taking a borrowed Buffer.\n\"\"\"\nunsafe extern \"C\" verb inspect(abs view: Buffer);\n\"\"\"\ncontract:\npurpose:\n    Take ownership of a Buffer and release it with drop.\ninputs:\n    packet: owned Buffer that is dropped.\noutputs:\n    No value.\nownership:\n    The buffer is moved into the verb and is dropped here, so the caller cannot use it afterwards.\nside_effects:\n    Releases the buffer storage.\n\"\"\"\nverb consume(dat packet: Buffer) { drop(packet); }\n\"\"\"Entry point that moves a buffer into consume and then passes the moved buffer to inspect, so the compiler must report the use of a moved value; it returns 0.\"\"\"\nverb main() -> Int { erg buffer = Buffer[1]; consume(packet: dat buffer); inspect(view: abs buffer); return 0; }\n",
            "E1005",
        ),
        (
            "exclusive-loan-alias",
            "\"\"\"Takes two mutable integers and does nothing with them; it exists so that a call can pass two borrows at once.\"\"\"\nverb merge_slots(ins left: Int, ins right: Int) { }\n\"\"\"Entry point that passes two different elements of one array as the two mutable arguments of merge_slots and returns 0.\"\"\"\nverb main() -> Int { erg values: Array[Int, 4] = Array[Int, 4](); merge_slots(left: ins values[0], right: ins values[1]); return 0; }\n",
            "E1065",
        ),
        (
            "missing-module",
            "import { open missing::module };\n\"\"\"Entry point that returns 0 without doing any work.\"\"\"\nverb main() -> Int { return 0; }\n",
            "E1101",
        ),
    ];
    for (fixture, source, expected_code) in fixtures {
        for backend in BACKENDS {
            let root = fixture_root(&format!("contract-failure-{fixture}"), backend);
            let input = write_fixture(&root, backend, None, None, source);
            let output = root.join("main.o");
            let build = build(&root, &input, &output, "obj");
            assert!(!build.status.success(), "{backend} must reject {fixture}");
            let diagnostics = format!(
                "{}{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
            assert!(
                diagnostics.contains(expected_code),
                "{backend} changed {fixture} diagnostic: {diagnostics}"
            );
            assert!(!output.exists(), "{backend} emitted an object for {fixture}");
            fs::remove_dir_all(root).expect("remove contract failure fixture");
        }
    }
}

#[test]
fn hosted_cli_backends_lower_directory_drop_cleanup_identically() {
    let source = "import { open std::fs, open std::path, };\n\"\"\"\ncontract:\npurpose:\n    Build a Path for the current directory from the ASCII bytes of a dot.\noutputs:\n    Result.Ok with the Path, or Result.Err with a PathError.\nownership:\n    The raw buffer is moved into the path conversion.\nerrors:\n    Any PathError from path_from_ascii is returned unchanged.\n\"\"\"\nverb current() -> Result[Path, PathError] { erg raw: Buffer = Buffer[0]; append(raw, 46u8); append(raw, 0u8); return path_from_ascii(storage: dat raw); }\n\"\"\"\ncontract:\npurpose:\n    Open the current directory for reading and report which stage failed.\noutputs:\n    0 when the directory opened, 1 when the path could not be built, 2 when the directory could not be opened.\nownership:\n    The path is borrowed by read_dir; the returned directory is dropped.\nerrors:\n    Path and read_dir errors map to the exit codes 1 and 2.\nside_effects:\n    Opens a directory with blocking filesystem access.\n\"\"\"\nmeta effect(\"filesystem\", \"blocking\") verb main() -> Int { erg current_path = current(); return case dat current_path { Result.Err(_) => 1, Result.Ok(path) => { erg opened = read_dir(path: abs path); return case dat opened { Result.Err(_) => 2, Result.Ok(_) => 0, }; }, }; }\n";
    let mut statuses = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("directory-drop", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} directory-drop build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        statuses.push(Command::new(&output).status().expect("run directory-drop fixture").code());
        fs::remove_dir_all(root).expect("remove directory-drop parity fixture");
    }
    assert_eq!(statuses, vec![Some(0), Some(0)]);
}

#[test]
fn hosted_cli_backends_preserve_bounded_lexicographic_directory_order() {
    let source = "import { open std::fs, open std::path, };\n\"\"\"\ncontract:\npurpose:\n    Build a Path for the relative directory named entries from its ASCII bytes.\noutputs:\n    Result.Ok with the Path, or Result.Err with a PathError.\nownership:\n    The raw buffer is moved into the path conversion.\nerrors:\n    Any PathError from path_from_ascii is returned unchanged.\n\"\"\"\nverb entries_path() -> Result[Path, PathError] { erg raw: Buffer = Buffer[0]; append(raw, 101u8); append(raw, 110u8); append(raw, 116u8); append(raw, 114u8); append(raw, 105u8); append(raw, 101u8); append(raw, 115u8); append(raw, 0u8); return path_from_ascii(storage: dat raw); }\n\"\"\"\ncontract:\npurpose:\n    Read two directory entries in order and check that the first name starts with a and the second with z.\ninputs:\n    directory: exclusive ins loan of Directory opened with a chosen entry order.\noutputs:\n    0 when the order is a then z, 3 or 5 on a read error, 4 or 6 when an entry is missing, 7 when the order is wrong.\nownership:\n    The directory is an exclusive ins loan that is advanced and then closed; each entry is dropped after its first byte is read.\nerrors:\n    next_dir_entry errors map to 3 and 5; a missing entry maps to 4 and 6.\nside_effects:\n    Reads entries from the directory and closes it, with blocking filesystem access.\n\"\"\"\nmeta effect(\"filesystem\", \"blocking\") verb check_order(ins directory: Directory) -> Int { erg first = next_dir_entry(directory: ins directory); return case dat first { Result.Err(_) => 3, Result.Ok(value) => case dat value { Option.None => 4, Option.Some(first_entry) => { erg first_byte: u8 = first_entry.name.storage[0]; drop(first_entry); erg second = next_dir_entry(directory: ins directory); return case dat second { Result.Err(_) => 5, Result.Ok(next) => case dat next { Option.None => 6, Option.Some(second_entry) => { erg second_byte: u8 = second_entry.name.storage[0]; drop(second_entry); close_dir(directory: ins directory); if first_byte == 97u8 && second_byte == 122u8 { return 0; } return 7; }, }, }; }, }, }; }\n\"\"\"\ncontract:\npurpose:\n    Open the entries directory with lexicographic ordering and verify the order of its first two entries.\noutputs:\n    0 on success, 1 when the path cannot be built, 2 when the directory cannot be opened, otherwise the status of check_order.\nownership:\n    The path is borrowed by read_dir_ordered; the directory is handed to check_order.\nerrors:\n    Path and read_dir_ordered errors map to the exit codes 1 and 2.\nside_effects:\n    Opens and reads a directory with blocking filesystem access.\n\"\"\"\nmeta effect(\"filesystem\", \"blocking\") verb main() -> Int { erg path = entries_path(); return case dat path { Result.Err(_) => 1, Result.Ok(value) => { erg selected_order = DirectoryOrder.Lexicographic; erg opened = read_dir_ordered(path: abs value, order: erg selected_order); return case dat opened { Result.Err(_) => 2, Result.Ok(directory) => check_order(directory: ins directory), }; }, }; }\n";
    for backend in BACKENDS {
        let root = fixture_root("ordered-directory", backend);
        fs::create_dir_all(root.join("entries")).expect("create ordered directory fixture");
        fs::write(root.join("entries").join("zeta"), b"zeta").expect("write zeta directory entry");
        fs::write(root.join("entries").join("alpha"), b"alpha")
            .expect("write alpha directory entry");
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} ordered-directory build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        for _ in 0..2 {
            let status = Command::new(&output)
                .current_dir(&root)
                .status()
                .expect("run ordered-directory parity fixture");
            assert_eq!(status.code(), Some(0), "{backend} ordered-directory result");
        }
        fs::remove_dir_all(root).expect("remove ordered directory parity fixture");
    }
}

#[test]
fn hosted_cli_backends_accept_named_array_capacity_constants() {
    let source = "const CAPACITY: Usize = 8u64 as Usize;\n\"\"\"Entry point that stores 41 in the first element of a u8 array sized by the CAPACITY constant and returns that element as Int plus 1.\"\"\"\nverb main() -> Int { erg values: Array[u8, CAPACITY] = Array[u8, CAPACITY](); values[0] = 41u8; return values[0] as Int + 1; }\n";
    for backend in BACKENDS {
        let root = fixture_root("named-array-capacity", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} named array capacity build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status =
            Command::new(&output).status().expect("run named array capacity parity fixture");
        assert_eq!(status.code(), Some(42), "{backend} named array capacity result");
        fs::remove_dir_all(root).expect("remove named array capacity fixture");
    }
}

#[test]
fn hosted_cli_backends_lower_qualified_imports_identically() {
    let source = "import config;\n\"\"\"Entry point that returns the config module constant LIMIT, converted to Int.\"\"\"\nverb main() -> Int { return config::LIMIT as Int; }\n";
    for backend in BACKENDS {
        let root = fixture_root("qualified-imports", backend);
        let input = write_fixture(&root, backend, None, None, source);
        fs::create_dir_all(root.join("src/config")).expect("create config module directory");
        fs::write(
            root.join("src/config/config.act"),
            "\"\"\"Re-export the constants of the config module.\"\"\"\nopen values;\n",
        )
        .expect("write config facade");
        fs::write(root.join("src/config/values.act"), "\"\"\"The limit that the qualified import fixture returns.\"\"\"\nopen const LIMIT: u8 = 30u8;\n")
            .expect("write config values");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} qualified import build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run qualified import fixture");
        assert_eq!(status.code(), Some(30), "{backend} qualified import result");
        fs::remove_dir_all(root).expect("remove qualified import fixture");
    }
}

#[test]
fn hosted_cli_backends_lower_qualified_enum_values_identically() {
    let source = "import colors;\n\"\"\"Entry point that builds the Red variant of the Color enum from the colors module and returns the score computed for it.\"\"\"\nverb main() -> Int { erg color: colors::Color = colors::Color.Red; return colors::score(value: erg color); }\n";
    for backend in BACKENDS {
        let root = fixture_root("qualified-enum", backend);
        let input = write_fixture(&root, backend, None, None, source);
        fs::create_dir_all(root.join("src/colors")).expect("create colors module directory");
        fs::write(
            root.join("src/colors/colors.act"),
            "\"\"\"Re-export the color declarations.\"\"\"\nopen api;\n",
        )
        .expect("write colors facade");
        fs::write(
            root.join("src/colors/api.act"),
            "\"\"\"A color of the fixture palette.\"\"\"\nopen enum Color {\n    \"\"\"The red color.\"\"\"\n    Red,\n    \"\"\"The blue color.\"\"\"\n    Blue,\n}\n\n\"\"\"Score a color: 42 for red, 1 for blue; the verb consumes the color.\"\"\"\nopen verb score(erg value: Color) -> Int { return case dat value { Color.Red => 42, Color.Blue => 1, }; }\n",
        )
        .expect("write colors API");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} qualified enum build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run qualified enum fixture");
        assert_eq!(status.code(), Some(42), "{backend} qualified enum result");
        fs::remove_dir_all(root).expect("remove qualified enum fixture");
    }
}

#[test]
fn hosted_cli_backends_lower_qualified_aggregate_constructors_identically() {
    let source = "import shapes;\n\"\"\"Entry point that builds a Packet from the shapes module with value 42 and returns its value field.\"\"\"\nverb main() -> Int { erg packet: shapes::Packet = shapes::Packet { value: 42, }; return packet.value; }\n";
    for backend in BACKENDS {
        let root = fixture_root("qualified-aggregate", backend);
        let input = write_fixture(&root, backend, None, None, source);
        fs::create_dir_all(root.join("src/shapes")).expect("create shapes module directory");
        fs::write(
            root.join("src/shapes/shapes.act"),
            "\"\"\"Re-export the packet declarations.\"\"\"\nopen api;\n",
        )
        .expect("write shapes facade");
        fs::write(root.join("src/shapes/api.act"), "\"\"\"A packet that carries one integer.\"\"\"\nopen struct Packet {\n    \"\"\"The carried integer.\"\"\"\n    value: Int,\n}\n")
            .expect("write shapes API");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} qualified aggregate build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run qualified aggregate fixture");
        assert_eq!(status.code(), Some(42), "{backend} qualified aggregate result");
        fs::remove_dir_all(root).expect("remove qualified aggregate fixture");
    }
}

#[test]
fn hosted_cli_backends_lower_qualified_generic_calls_identically() {
    let source = "import generic;\n\"\"\"Entry point that returns 41 passed through the identity function of the generic module, instantiated for Int.\"\"\"\nverb main() -> Int { erg value: Int = 41; return generic::identity[Int](value: erg value); }\n";
    for backend in BACKENDS {
        let root = fixture_root("qualified-generic", backend);
        let input = write_fixture(&root, backend, None, None, source);
        fs::create_dir_all(root.join("src/generic")).expect("create generic module directory");
        fs::write(
            root.join("src/generic/generic.act"),
            "\"\"\"Re-export the generic identity verb.\"\"\"\nopen api;\n",
        )
        .expect("write generic facade");
        fs::write(
            root.join("src/generic/api.act"),
            "\"\"\"Return the given value unchanged; the verb returns the owner it received.\"\"\"\nopen verb identity[T](erg value: T) -> T { return value; }\n",
        )
        .expect("write generic API");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} qualified generic build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run qualified generic fixture");
        assert_eq!(status.code(), Some(41), "{backend} qualified generic result");
        fs::remove_dir_all(root).expect("remove qualified generic fixture");
    }
}

#[test]
fn hosted_cli_backends_share_float_constants_and_comparisons() {
    let source = "\"\"\"Entry point that compares an f32 temperature of 21.5 and an f64 pressure of 1013.25 against their limits and returns 42 when both hold, otherwise 1.\"\"\"\nverb main() -> Int { erg temperature: f32 = 21.5; erg pressure: f64 = 1013.25; return case temperature < 22.0f32 && pressure > 1000.0 { true => 42, _ => 1, }; }\n";
    for backend in BACKENDS {
        let root = fixture_root("float-hosted", backend);
        let input = write_fixture(&root, backend, None, None, source);
        let manifest = root.join("Actus.toml");
        let contents = fs::read_to_string(&manifest).expect("read float parity manifest");
        fs::write(
            &manifest,
            contents.replace("verify_no_float_ir = true", "verify_no_float_ir = false"),
        )
        .expect("disable zero-float audit for hosted float parity");
        let output = root.join("main");
        let build = build(&root, &input, &output, "exe");
        assert!(
            build.status.success(),
            "{backend} hosted float build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let status = Command::new(&output).status().expect("run hosted float parity fixture");
        assert_eq!(status.code(), Some(42), "{backend} hosted float runtime result");
        fs::remove_dir_all(root).expect("remove hosted float fixture");
    }
}

#[test]
fn freestanding_cli_backends_preserve_symbols_and_zero_float_evidence() {
    let mut defined = Vec::new();
    let mut undefined = Vec::new();
    for backend in BACKENDS {
        let root = fixture_root("freestanding", backend);
        let input = write_fixture(&root, backend, None, Some("freestanding"), SOURCE);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(
            build.status.success(),
            "{backend} freestanding build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let stdout = String::from_utf8_lossy(&build.stdout);
        let stderr = String::from_utf8_lossy(&build.stderr);
        assert!(
            stdout.contains("no floating-point instructions")
                || stderr.contains("no floating-point instructions"),
            "{backend} did not report zero-float verification; stdout={stdout} stderr={stderr}"
        );
        let bytes = fs::read(&output).expect("read freestanding parity object");
        defined.push(symbols(&bytes, true));
        undefined.push(referenced_undefined_symbols(&bytes));
        fs::remove_dir_all(root).expect("remove freestanding parity fixture");
    }
    assert!(defined[0].contains("boot"), "freestanding entry symbol missing: {:?}", defined[0]);
    assert_eq!(defined[0], defined[1], "freestanding exported symbols diverged");
    assert!(
        undefined[0].contains("actus_region_drop") && undefined[0].contains("actus_region_open"),
        "required Region runtime bridges are missing: {:?}",
        undefined[0]
    );
    assert_eq!(undefined[0], undefined[1], "runtime bridge relocations diverged");
}

#[test]
fn freestanding_cli_backends_share_large_aggregate_failure_class() {
    let source = "struct Cell { erg value: u8, }\n\"\"\"Boot verb that creates an array of 131072 Cell structs and returns the value of the first cell as Int.\"\"\"\nverb boot() -> Int { erg cells: Array[Cell, 131072] = Array[Cell, 131072](); return cells[0].value as Int; }\n";
    for backend in BACKENDS {
        let root = fixture_root("failure", backend);
        let input = write_fixture(&root, backend, Some("thumbv7em-none-eabihf"), None, source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(!build.status.success(), "{backend} must reject implicit allocation");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(
            diagnostics.contains("implicit dynamic allocation")
                || diagnostics.contains("implicit heap allocation"),
            "{backend} failure class changed: {diagnostics}"
        );
        assert!(!output.exists(), "{backend} emitted an object after allocation rejection");
        fs::remove_dir_all(root).expect("remove failure parity fixture");
    }
}

#[test]
fn freestanding_cli_backends_reject_float_at_shared_target_boundary() {
    let source = "\"\"\"Boot verb that returns 0 when the f32 sample 1.5 is greater than 1.0, and 1 otherwise.\"\"\"\nverb boot() -> Int { erg sample: f32 = 1.5f32; return case sample > 1.0f32 { true => 0, _ => 1, }; }\n";
    for backend in BACKENDS {
        let root = fixture_root("float-freestanding", backend);
        let input = write_fixture(&root, backend, None, Some("freestanding"), source);
        let output = root.join("main.o");
        let build = build(&root, &input, &output, "obj");
        assert!(!build.status.success(), "{backend} must reject freestanding float");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        assert!(
            diagnostics
                .contains("floating-point values are forbidden by the freestanding target profile"),
            "{backend} float policy diagnostic changed: {diagnostics}"
        );
        assert!(!output.exists(), "{backend} emitted an object after float rejection");
        fs::remove_dir_all(root).expect("remove freestanding float fixture");
    }
}

#[test]
fn freestanding_cli_backends_reject_hosted_services_before_codegen() {
    let services = [
        (
            "fs",
            "import { open std::fs };\n\"\"\"Boot verb that returns 0 and does nothing else.\"\"\"\nverb boot() -> Int { return 0; }\n",
        ),
        (
            "time",
            "import { open std::time };\n\"\"\"Boot verb that returns 0 and does nothing else.\"\"\"\nverb boot() -> Int { return 0; }\n",
        ),
        (
            "process",
            "import { open std::process };\n\"\"\"Boot verb that returns 0 and does nothing else.\"\"\"\nverb boot() -> Int { return 0; }\n",
        ),
    ];
    for (service, source) in services {
        for backend in BACKENDS {
            let root = fixture_root(&format!("hosted-service-failure-{service}"), backend);
            let input = write_fixture(&root, backend, Some("thumbv7em-none-eabihf"), None, source);
            let output = root.join("main.o");
            let build = build(&root, &input, &output, "obj");
            assert!(!build.status.success(), "{backend} must reject hosted std::{service}");
            let diagnostics = format!(
                "{}{}",
                String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr)
            );
            assert!(
                diagnostics.contains("E1112"),
                "{backend} changed std::{service} target rejection: {diagnostics}"
            );
            assert!(!output.exists(), "{backend} emitted an object for hosted std::{service}");
            fs::remove_dir_all(root).expect("remove hosted-service failure fixture");
        }
    }
}
