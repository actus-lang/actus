; Integer-only Thumb object fixture for bundled-toolchain verification.
; eabihf permits VFP instructions, but this fixture must not generate them.
target triple = "thumbv7em-none-eabihf"

%ThirtyTwoBytes = type { i64, i64, i64, i64 }

define i64 @actus_thumb_integer_only(i64 %left, i64 %right) {
entry:
  %sum = add i64 %left, %right
  ret i64 %sum
}

define void @actus_thumb_copy_32_bytes(ptr %destination, ptr %source) {
entry:
  %first = getelementptr %ThirtyTwoBytes, ptr %source, i32 0, i32 0
  %second = getelementptr %ThirtyTwoBytes, ptr %source, i32 0, i32 1
  %third = getelementptr %ThirtyTwoBytes, ptr %source, i32 0, i32 2
  %fourth = getelementptr %ThirtyTwoBytes, ptr %source, i32 0, i32 3
  %first_value = load i64, ptr %first, align 8
  %second_value = load i64, ptr %second, align 8
  %third_value = load i64, ptr %third, align 8
  %fourth_value = load i64, ptr %fourth, align 8
  %destination_first = getelementptr %ThirtyTwoBytes, ptr %destination, i32 0, i32 0
  %destination_second = getelementptr %ThirtyTwoBytes, ptr %destination, i32 0, i32 1
  %destination_third = getelementptr %ThirtyTwoBytes, ptr %destination, i32 0, i32 2
  %destination_fourth = getelementptr %ThirtyTwoBytes, ptr %destination, i32 0, i32 3
  store i64 %first_value, ptr %destination_first, align 8
  store i64 %second_value, ptr %destination_second, align 8
  store i64 %third_value, ptr %destination_third, align 8
  store i64 %fourth_value, ptr %destination_fourth, align 8
  ret void
}
