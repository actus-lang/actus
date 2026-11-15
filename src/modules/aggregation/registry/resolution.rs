pub(super) fn is_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "Result"
            | "Option"
            | "Arena"
            | "Box"
            | "Array"
            | "Buffer"
            | "Int"
            | "Bool"
            | "String"
            | "Usize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
    )
}
