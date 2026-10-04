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
fn preserves_generic_array_type_when_passing_a_struct_field() {
    build_and_run(
        "struct Rule { erg value: u32, } struct Vocabulary { erg rules: Array[Rule, 2], } verb inspect(abs rules: Array[Rule, 2]) -> u32 { return rules[0].value; } verb main() -> Int { erg vocabulary: Vocabulary = Vocabulary { rules: Array[Rule, 2](), }; vocabulary.rules[0].value = 41u32; return inspect(rules: abs vocabulary.rules) as Int; }",
        "generic-array-struct-field-argument",
        41,
    );
}

#[cfg(unix)]
#[test]
fn copies_inline_array_struct_fields_across_native_return_and_call_boundaries() {
    build_and_run(
        "struct Snapshot { erg magic: u32, erg columns: Array[u8, 4], } verb make() -> Snapshot { erg snapshot: Snapshot = Snapshot { magic: 42u32, columns: Array[u8, 4](), }; snapshot.columns[0] = 41u8; return snapshot; } verb inspect(abs snapshot: Snapshot) -> Int { return snapshot.columns[0] as Int + snapshot.magic as Int; } verb main() -> Int { erg snapshot: Snapshot = make(); return inspect(snapshot: abs snapshot); }",
        "inline-array-struct-return",
        83,
    );
}

#[cfg(unix)]
#[test]
fn transfers_an_owned_enum_return_without_double_drop() {
    build_and_run(
        "verb produce() -> Option[u32] { erg result: Option[u32] = Option[u32].None; result = Option[u32].Some(1u32); return result; } verb main() -> Int { erg result: Option[u32] = produce(); return 42; }\n",
        "owned-enum-return",
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
fn executes_struct_field_compound_assignment() {
    build_and_run(
        "struct Point { x: Int, y: Int, } verb main() -> Int { erg point = Point { x: 5, y: 0, }; point.x += 37; return point.x; }\n",
        "struct-compound-assignment",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_pack_field_compound_assignment() {
    build_and_run(
        "pack Register { erg storage: u8; layout little; fields { erg value: u8 at 0; } } verb main() -> Int { erg register = Register { storage: 5u8, }; register.value += 37u8; return register.value as Int; }\n",
        "pack-compound-assignment",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_nested_struct_access_and_owned_drop() {
    build_and_run(
        "struct Inner { x: Int, y: Int, } struct Holder { inner: Inner, erg payload: Buffer, } verb main() -> Int { erg holder = Holder { inner: Inner { x: 40, y: 2, }, payload: Buffer[4], }; return holder.inner.x + holder.inner.y; }\n",
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
fn executes_static_performance_dispatch() {
    build_and_run(
        "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return self.value + 1; } } verb main() -> Int { erg source = File { value: 41, }; abs file = ref source; return file.write(); }\n",
        "static-performance-dispatch",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_direct_dat_method_dispatch() {
    build_and_run(
        "struct File { value: Int, } verb consume(dat self: File) -> Int { return self.value; } verb main() -> Int { erg file = File { value: 42, }; return file.consume(); }\n",
        "direct-dat-method-dispatch",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_dynamic_performance_dispatch() {
    build_and_run(
        "struct File { value: Int, } role Writer { verb write(abs self: File) -> Int; } perform Writer for File { verb write(abs self: File) -> Int { return self.value + 1; } } verb send(abs writer: dynamic Writer) -> Int { return writer.write(); } verb main() -> Int { erg file = File { value: 41, }; return send(writer: ref file); }\n",
        "dynamic-performance-dispatch",
        42,
    );
}

#[cfg(unix)]
#[test]
fn cleans_up_owned_field_in_monomorphized_generic_struct() {
    build_and_run(
        "struct Box[T] { erg item: T, } verb main() -> Int { erg boxed = Box[Buffer] { item: Buffer[4], }; return 42; }\n",
        "generic-owned-field",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_monomorphized_generic_enum_cleanup() {
    build_and_run(
        "enum Box[T] { Some(T), None, } verb main() -> Int { erg item = Box[Buffer].Some(Buffer[4]); return case dat item { Box.Some(payload) => { drop(payload); return 42; }, Box.None => 0, }; }\n",
        "generic-enum-cleanup",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_try_unwrap_in_an_initializer() {
    build_and_run(
        "verb produce() -> Result[Int, Int] { return Ok(42); } verb worker() -> Result[Int, Int] { erg value = produce()?; return Ok(value); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(value) => value, Result.Err(_) => 1, }; }\n",
        "try-initializer",
        42,
    );
}

#[cfg(unix)]
#[test]
fn preserves_struct_ok_payload_through_try_unwrap() {
    build_and_run(
        "struct Payload { value: Int, } verb make_payload() -> Payload { return Payload { value: 42, }; } verb produce() -> Result[Payload, Int] { return Ok(make_payload()); } verb worker() -> Result[Payload, Int] { erg payload = produce()?; return Ok(payload); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(payload) => payload.value, Result.Err(_) => 1, }; }\n",
        "try-struct-ok-payload",
        42,
    );
}

#[cfg(unix)]
#[test]
fn preserves_struct_err_payload_through_try_propagation() {
    build_and_run(
        "struct Failure { code: Int, } verb produce() -> Result[Int, Failure] { return Err(Failure { code: 42, }); } verb worker() -> Result[Int, Failure] { produce()?; return Ok(0); } verb main() -> Int { erg result = worker(); return case dat result { Result.Ok(_) => 1, Result.Err(error) => error.code, }; }\n",
        "try-struct-err-payload",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_option_none_without_null_propagation() {
    build_and_run(
        "verb main() -> Int { erg option = Option[Int].None; return case dat option { Option.Some(_) => 1, Option.None => 0, }; }\n",
        "option-none",
        0,
    );
}

#[cfg(unix)]
#[test]
fn executes_result_err_without_exception_propagation() {
    build_and_run(
        "verb main() -> Int { erg result = Result[Int, String].Err(\"error\"); return case dat result { Result.Ok(_) => 1, Result.Err(message) => { drop(message); return 0; }, }; }\n",
        "result-err",
        0,
    );
}

#[cfg(unix)]
#[test]
fn executes_try_result_success_and_error_paths() {
    build_and_run(include_str!("../examples/try_result.act"), "try-result", 42);
}

#[cfg(unix)]
#[test]
fn executes_short_result_constructors_natively() {
    build_and_run(
        "enum IoError { Failed, } verb produce() -> Result[Int, IoError] { return Ok(42); } verb main() -> Int { erg result = produce(); return case dat result { Result.Ok(value) => value, Result.Err(_) => 0, }; }\n",
        "short-result",
        42,
    );
}

#[cfg(unix)]
#[test]
fn executes_explicit_result_err_with_user_enum_natively() {
    build_and_run(
        "enum Failure { Failed, } verb main() -> Int { erg result = Result[Int, Failure].Err(Failure.Failed); return case dat result { Result.Ok(_) => 1, Result.Err(error) => case dat error { Failure.Failed => 0, }, }; }\n",
        "explicit-result-err-user-enum",
        0,
    );
}

#[cfg(unix)]
#[test]
fn avoids_double_drop_after_moving_owned_field() {
    build_and_run(
        "struct Holder { erg payload: Buffer, value: Int, } verb consume(dat payload: Buffer) -> Int { drop(payload); return 0; } verb main() -> Int { erg holder = Holder { payload: Buffer[4], value: 42, }; consume(payload: holder.payload); return 42; }\n",
        "partial-move",
        42,
    );
}

#[cfg(unix)]
#[test]
fn replaces_a_struct_assignment_after_cleaning_the_previous_owner() {
    let root = std::env::temp_dir().join(format!("actus-struct-replace-{}", std::process::id()));
    let input = root.with_extension("act");
    let output = root.with_extension("bin");
    let source = "struct Counter { value: Int, } perform Drop for Counter { verb drop(ins self: Counter) { print(self.value); } } verb main() -> Int { erg destination = Counter { value: 1, }; erg source = Counter { value: 2, }; destination = source; return 0; }";
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
    let execution = std::process::Command::new(&output).output().expect("run executable");
    assert_eq!(execution.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&execution.stdout), "1\n2\n");
    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}
