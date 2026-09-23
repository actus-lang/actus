#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use actus::cli::run_with_args;

#[cfg(unix)]
fn build_and_run(source: &str, name: &str, expected: i32) {
    let root = std::env::temp_dir().join(format!("actus-cli-{name}-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    fs::write(&input, source).expect("write source");
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
    let status = std::process::Command::new(&output).status().expect("run executable");
    assert_eq!(status.code(), Some(expected));
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[cfg(unix)]
#[test]
fn executes_struct_field_access() {
    build_and_run(
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg point = Point { x: 40, y: 2, }; return point.x + point.y; }\n",
        "struct-access",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_struct_field_assignment() {
    build_and_run(
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg point = Point { x: 40, y: 2, }; point.x = 41; return point.x + point.y; }\n",
        "struct-assignment",
        43,
    );
}

#[cfg(unix)]
#[test]
fn executes_nested_struct_access_and_owned_drop() {
    build_and_run(
        "struct Inner { x: Int, y: Int, } struct Holder { inner: Inner, erg payload: Buffer, } verb main() -> Int { erg holder = Holder { inner: Inner { x: 40, y: 2, }, payload: allocate(4), }; return holder.inner.x + holder.inner.y; }\n",
        "nested-struct",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_nested_struct_field_assignment() {
    build_and_run(
        "struct Point { x: Int, y: Int, } struct Player { position: Point, score: Int, } verb main() -> Int { erg player = Player { position: Point { x: 3, y: 4, }, score: 5, }; player.position.x = 8; return player.position.x + player.position.y + player.score; }\n",
        "nested-struct-assignment",
        17,
    );
}

#[cfg(unix)]
#[test]
fn executes_monomorphized_generic_struct() {
    build_and_run(
        "struct Box[T] { item: T, } verb main() -> Int { erg boxed = Box[Int] { item: 42, }; return boxed.item; }\n",
        "generic-struct",
        42,
    );
}

#[cfg(unix)]
#[test]
fn cleans_up_owned_field_in_monomorphized_generic_struct() {
    build_and_run(
        "struct Box[T] { erg item: T, } verb main() -> Int { erg boxed = Box[Buffer] { item: allocate(4), }; return 42; }\n",
        "generic-owned-field",
        42,
    );
}

#[cfg(unix)]
#[test]
fn avoids_double_drop_after_moving_owned_field() {
    build_and_run(
        "struct Holder { erg payload: Buffer, value: Int, } verb consume(dat payload: Buffer) -> Int { drop(payload); return 0; } verb main() -> Int { erg holder = Holder { payload: allocate(4), value: 42, }; consume(payload: holder.payload); return 42; }\n",
        "partial-move",
        42,
    );
}
