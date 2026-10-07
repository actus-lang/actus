use std::fs;
use std::process::Command;
use std::time::Instant;

const CASES: &[(u64, &str)] =
    &[(1_048_576u64, "1MiB"), (1_073_741_824u64, "1GiB"), (1_099_511_627_776u64, "1TiB")];

fn run_case(
    logical_length: u64,
    label: &str,
    element_type: &str,
    resident_bytes: u64,
    declaration: &str,
) {
    let root = std::env::temp_dir()
        .join(format!("actus-region-scale-{element_type}-{label}-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("region-scale-example");
    fs::create_dir_all(&source_root).expect("scale fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_region_scale_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("scale fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        format!(
            "{declaration}import std::region; verb main() -> Int {{ erg backing: Buffer = Buffer[{resident_bytes}]; erg logical_length: u64 = {logical_length}u64; erg window_start: u64 = 0u64; erg window_count: u64 = 1u64; erg opened = region_open[{element_type}](backing: dat backing, logical_length: erg logical_length, window_start: erg window_start, window_count: erg window_count); return case dat opened {{ Result.Err(_) => 1, Result.Ok(region) => {{ erg destination: Buffer = Buffer[{resident_bytes}]; erg index: u64 = 0u64; erg read = region_read(region: abs region, index: erg index, destination: ins destination); return case dat read {{ Result.Err(_) => 2, Result.Ok(_) => 0, }}; }}, }}; }}\n"
        ),
    )
    .expect("scale fixture source should be written");

    let started = Instant::now();
    let build = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .output()
        .expect("Actus scale build should start");
    let build_ms = started.elapsed().as_millis();
    assert!(
        build.status.success(),
        "scale build failed for {label}: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let executable_bytes = fs::metadata(&output).expect("read scale executable metadata").len();
    let started = Instant::now();
    let execution = Command::new(&output).output().expect("scale executable should run");
    let run_us = started.elapsed().as_micros();
    assert_eq!(execution.status.code(), Some(0), "scale execution failed for {label}");
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    println!(
        "REGION_SCALE element={element_type} logical_label={label} logical_bytes={logical_length} resident_bytes={resident_bytes} build_ms={build_ms} executable_bytes={executable_bytes} run_us={run_us} status=0"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn hosted_region_scales_logical_capacity_with_one_resident_byte() {
    for &(logical_length, label) in CASES {
        run_case(logical_length, label, "u8", 1, "");
    }
}

#[test]
fn hosted_packed_region_scales_logical_capacity_with_one_resident_element() {
    const DECLARATION: &str = "pack Minicolumn { erg storage: Array[u8, 64]; layout little; fields { erg marker: u8 at 0; abs _reserved_0: u128 at 8 = 0; abs _reserved_1: u128 at 136 = 0; abs _reserved_2: u128 at 264 = 0; abs _reserved_3: u120 at 392 = 0; } } ";
    for &(logical_length, label) in CASES {
        run_case(logical_length, label, "Minicolumn", 64, DECLARATION);
    }
}
