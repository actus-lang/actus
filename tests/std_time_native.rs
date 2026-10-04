use std::fs;
use std::process::Command;

use object::{Object, ObjectSymbol};

#[test]
fn hosted_std_time_bridge_builds_and_runs() {
    let root = std::env::temp_dir().join(format!("actus-std-time-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("time-example");
    let object_output = root.join("time-example.o");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_fixture\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg started: u64 = monotonic_nanos(); erg finished: u64 = monotonic_nanos(); erg elapsed: u64 = finished - started; if finished >= started && elapsed >= 0u64 { return 0; } return 1; }",
    )
    .expect("fixture source should be written");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "hosted std time build failed: {status}");

    let object_status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "obj",
            "-o",
            object_output.to_str().expect("object path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus object build should start");
    assert!(object_status.success(), "hosted std time object build failed: {object_status}");
    let object_bytes = fs::read(&object_output).expect("time object should be readable");
    let object_file =
        object::File::parse(object_bytes.as_slice()).expect("time object should parse");
    let bridge_symbols = object_file
        .symbols()
        .filter_map(|symbol| symbol.name().ok())
        .filter(|name| {
            name.trim_start_matches('_') == "actus_mod_3_std_4_time__verb_monotonic_5fnanos"
        })
        .count();
    assert_eq!(bridge_symbols, 1, "timer facade dependency must be registered exactly once");

    let execution = Command::new(&output).output().expect("time executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn typed_instant_and_duration_api_builds_and_runs() {
    let root = std::env::temp_dir().join(format!("actus-std-time-types-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("time-types-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_types\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg started: Instant = now(); erg finished: Instant = now(); erg raw_nanos: u64 = 42u64; erg span: Duration = duration_nanos(nanos: erg raw_nanos); erg start_ticks: u64 = instant_ticks(abs started); erg finish_ticks: u64 = instant_ticks(abs finished); erg span_nanos: u64 = duration_as_nanos(abs span); erg sleep_result = sleep(duration: abs span); return case dat sleep_result { Result.Ok(_) => if finish_ticks >= start_ticks && span_nanos == 42u64 { 0 } else { 1 }, Result.Err(_) => 2, }; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "typed time build failed: {status}");

    let execution = Command::new(&output).output().expect("typed time executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn monotonic_deadline_api_builds_and_runs() {
    let root = std::env::temp_dir().join(format!("actus-std-time-deadline-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("deadline-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_deadline\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg raw_nanos: u64 = 1000000000u64; erg span: Duration = duration_nanos(nanos: erg raw_nanos); erg start: Instant = now(); erg candidate = deadline_after(start: abs start, duration: abs span); return case dat candidate { Result.Ok(deadline) => if expired(deadline: abs deadline) { 1 } else { 0 }, Result.Err(_) => 2, }; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "deadline build failed: {status}");

    let execution = Command::new(&output).output().expect("deadline executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn checked_duration_units_and_arithmetic_run_natively() {
    let root = std::env::temp_dir().join(format!("actus-std-time-duration-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("duration-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_duration\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb unwrap_duration(erg candidate: Result[Duration, TimeError]) -> Duration { return case dat candidate { Result.Ok(value) => value, Result.Err(_) => Duration { nanos: 0u64, }, }; } verb main() -> Int { erg one: u64 = 1u64; erg two: u64 = 2u64; erg second_result = duration_seconds(seconds: erg one); erg millisecond_result = duration_milliseconds(milliseconds: erg one); erg microsecond_result = duration_microseconds(microseconds: erg one); erg one_second: Duration = unwrap_duration(candidate: erg second_result); erg one_millisecond: Duration = unwrap_duration(candidate: erg millisecond_result); erg one_microsecond: Duration = unwrap_duration(candidate: erg microsecond_result); erg sum_result = duration_add(left: abs one_second, right: abs one_millisecond); erg scaled_result = duration_mul(duration: abs one_microsecond, multiplier: two); erg sum: Duration = unwrap_duration(candidate: erg sum_result); erg scaled: Duration = unwrap_duration(candidate: erg scaled_result); erg nanos: u64 = duration_as_nanos(abs sum); erg scaled_nanos: u64 = duration_as_nanos(abs scaled); if nanos == 1001000000u64 && scaled_nanos == 2000u64 { return 0; } return 1; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "duration arithmetic build failed: {status}");

    let execution = Command::new(&output).output().expect("duration executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn freestanding_std_time_is_rejected_before_native_emission() {
    let root =
        std::env::temp_dir().join(format!("actus-std-time-freestanding-{}", std::process::id()));
    let source_root = root.join("src");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_freestanding\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\ntarget = \"x86_64-unknown-none\"\n",
    )
    .expect("fixture manifest should be written");
    fs::write(source_root.join("main.act"), "import std::time; verb main() -> Int { return 0; }")
        .expect("fixture source should be written");

    let check = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "check",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
        ])
        .output()
        .expect("Actus check should start");
    assert!(!check.status.success());
    assert!(String::from_utf8_lossy(&check.stderr).contains("E1112"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unsigned_elapsed_underflow_is_rejected_before_native_execution() {
    let root =
        std::env::temp_dir().join(format!("actus-std-time-underflow-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("underflow-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_underflow\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg started: u64 = monotonic_nanos(); erg finished: u64 = monotonic_nanos(); return (started - finished) as Int; }",
    )
    .expect("fixture source should be written");

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
        .status()
        .expect("Actus build should start");
    assert!(build.success(), "unsigned underflow fixture should compile: {build}");
    let execution = Command::new(&output).output().expect("underflow executable should run");
    assert!(!execution.status.success(), "unsigned elapsed underflow was accepted");
    assert_ne!(execution.status.code(), Some(0), "underflow returned success");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn duration_boundaries_and_typed_failures_run_natively() {
    let root =
        std::env::temp_dir().join(format!("actus-std-time-boundaries-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("duration-boundaries-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_boundaries\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb is_overflow(erg result: Result[Duration, TimeError]) -> Int { return case dat result { Result.Err(error) => case dat error { TimeError.Overflow => 0, _ => 1, }, Result.Ok(_) => 2, }; } verb is_precision_loss(erg result: Result[Duration, TimeError]) -> Int { return case dat result { Result.Err(error) => case dat error { TimeError.PrecisionLoss => 0, _ => 1, }, Result.Ok(_) => 2, }; } verb is_zero_divisor(erg result: Result[Duration, TimeError]) -> Int { return case dat result { Result.Err(error) => case dat error { TimeError.ZeroDivisor => 0, _ => 1, }, Result.Ok(_) => 2, }; } verb is_negative(erg result: Result[Duration, TimeError]) -> Int { return case dat result { Result.Err(error) => case dat error { TimeError.NegativeDuration => 0, _ => 1, }, Result.Ok(_) => 2, }; } verb is_exact(erg result: Result[Duration, TimeError]) -> Int { return case dat result { Result.Ok(value) => if duration_as_nanos(abs value) == 2u64 { 0 } else { 1 }, Result.Err(_) => 2, }; } verb main() -> Int { erg max_input: u64 = 18446744073u64; erg overflow_input: u64 = 18446744074u64; erg overflow_millis_input: u64 = 18446744073710u64; erg overflow_micros_input: u64 = 18446744073709552u64; erg divisor: u64 = 2u64; erg zero: u64 = 0u64; erg four: Duration = Duration { nanos: 4u64, }; erg five: Duration = Duration { nanos: 5u64, }; erg one: Duration = Duration { nanos: 1u64, }; erg two: Duration = Duration { nanos: 2u64, }; erg max_seconds = duration_seconds(seconds: erg max_input); erg overflow_seconds = duration_seconds(seconds: erg overflow_input); erg overflow_millis = duration_milliseconds(milliseconds: erg overflow_millis_input); erg overflow_micros = duration_microseconds(microseconds: erg overflow_micros_input); erg exact = duration_div(duration: abs four, divisor: erg divisor); erg non_exact = duration_div(duration: abs five, divisor: erg divisor); erg zero_divisor = duration_div(duration: abs four, divisor: erg zero); erg underflow = duration_sub(left: abs one, right: abs two); if is_overflow(result: erg overflow_seconds) != 0 || is_overflow(result: erg overflow_millis) != 0 || is_overflow(result: erg overflow_micros) != 0 || is_precision_loss(result: erg non_exact) != 0 || is_zero_divisor(result: erg zero_divisor) != 0 || is_negative(result: erg underflow) != 0 || is_exact(result: erg exact) != 0 || is_overflow(result: erg max_seconds) != 2 { return 1; } return 0; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "duration boundary build failed: {status}");

    let execution =
        Command::new(&output).output().expect("duration boundary executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn deadline_overflow_is_rejected_natively() {
    let root = std::env::temp_dir()
        .join(format!("actus-std-time-deadline-overflow-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("deadline-overflow-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_deadline_overflow\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb is_overflow(erg result: Result[Deadline, TimeError]) -> Int { return case dat result { Result.Err(error) => case dat error { TimeError.Overflow => 0, _ => 1, }, Result.Ok(_) => 2, }; } verb main() -> Int { erg start: Instant = Instant { ticks: 18446744073709551615u64, }; erg duration: Duration = Duration { nanos: 1u64, }; erg result = deadline_after(start: abs start, duration: abs duration); return is_overflow(result: erg result); }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "deadline overflow build failed: {status}");

    let execution =
        Command::new(&output).output().expect("deadline overflow executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn zero_duration_busy_wait_delay_runs_natively() {
    let root = std::env::temp_dir().join(format!("actus-std-time-delay-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("delay-example");
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_delay\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(
        source_root.join("main.act"),
        "import std::time; verb main() -> Int { erg zero: Duration = Duration { nanos: 0u64, }; erg result = delay(duration: abs zero); return case dat result { Result.Ok(_) => 0, Result.Err(_) => 1, }; }",
    )
    .expect("fixture source should be written");

    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "busy-wait delay build failed: {status}");

    let execution = Command::new(&output).output().expect("delay executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn timer_state_machine_and_missed_tick_policies_run_natively() {
    let root = std::env::temp_dir().join(format!("actus-std-time-timer-{}", std::process::id()));
    let source_root = root.join("src");
    let output = root.join("timer-example");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source_root).expect("fixture source directory should be created");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"std_time_timer\"\nversion = \"0.1.0\"\nedition = \"alpha\"\nentry = \"main\"\n\n[build]\nruntime = \"std\"\nverify_no_float_ir = true\n",
    )
    .expect("fixture manifest should be written");
    fs::write(source_root.join("main.act"), include_str!("fixtures/std_time_timer.act"))
        .expect("timer fixture source should be written");
    let status = Command::new(env!("CARGO_BIN_EXE_actus"))
        .args([
            "build",
            source_root.join("main.act").to_str().expect("fixture path should be valid UTF-8"),
            "--strict",
            "--emit",
            "exe",
            "-o",
            output.to_str().expect("output path should be valid UTF-8"),
        ])
        .status()
        .expect("Actus build should start");
    assert!(status.success(), "timer build failed: {status}");

    let execution = Command::new(&output).output().expect("timer executable should run");
    assert_eq!(execution.status.code(), Some(0));
    assert!(execution.stdout.is_empty());
    assert!(execution.stderr.is_empty());
    let _ = fs::remove_dir_all(root);
}
