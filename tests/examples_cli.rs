#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn sensor_telemetry_example_builds_and_executes() {
    let root = std::env::temp_dir().join(format!("actus-sensor-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/sensor_telemetry.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run sensor telemetry example");
    assert_eq!(status.code(), Some(0));
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn hardware_register_example_builds_and_executes() {
    let root = std::env::temp_dir().join(format!("actus-register-example-{}", std::process::id()));
    let output = root.with_extension("bin");
    let source = format!("{}/examples/hardware_register.act", env!("CARGO_MANIFEST_DIR"));
    let result = run_with_args(
        vec![
            "build".to_owned(),
            source,
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status =
        std::process::Command::new(&output).status().expect("run hardware register example");
    assert_eq!(status.code(), Some(8));
    let _ = fs::remove_file(output);
}
