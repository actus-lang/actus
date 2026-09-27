#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
#[test]
fn executes_pack_field_reads_and_erg_mutation_natively() {
    let root = std::env::temp_dir().join(format!("actus-pack-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = r#"
pack Control {
    erg storage: u32;
    layout little;
    fields {
        erg enabled: u1 at 0;
        abs mode: u3 at 1;
        erg channel: u5 at 4;
        abs _reserved: u23 at 9;
    }
}

pack BigControl {
    erg storage: u8;
    layout big;
    fields {
        erg high: u4 at 0;
        abs low: u4 at 4;
    }
}

verb main() -> Int {
    erg control = Control { storage: 0, };
    control.enabled = 1;
    control.channel = 21;
    erg big = BigControl { storage: 0, };
    big.high = 10;
    return control.enabled + control.channel + control.mode + big.high;
}
"#;
    fs::write(&input, source).expect("write pack fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run pack fixture");
    assert_eq!(status.code(), Some(32));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn executes_big_endian_network_frame_layout_natively() {
    let root = std::env::temp_dir().join(format!("actus-network-frame-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = r#"
pack NetworkHeader {
    erg storage: u32;
    layout big;
    fields {
        erg version: u4 at 0;
        abs header_length: u4 at 4;
        erg total_length: u8 at 8;
        abs _reserved: u16 at 16 = 0;
    }
}

verb main() -> Int {
    erg header = NetworkHeader { storage: 0, };
    header.version = 4;
    header.total_length = 150;
    return header.version + header.total_length;
}
"#;
    fs::write(&input, source).expect("write network frame fixture");
    let result = run_with_args(
        vec![
            "build".to_owned(),
            input.display().to_string(),
            "--emit".to_owned(),
            "exe".to_owned(),
            "-o".to_owned(),
            output.display().to_string(),
        ]
        .into_iter(),
    );
    assert_eq!(result, 0);
    let status = std::process::Command::new(&output).status().expect("run network frame fixture");
    assert_eq!(status.code(), Some(154));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
