pub(super) fn mask(width: u8) -> String {
    let mask = if width == 128 { u128::MAX } else { (1u128 << width) - 1 };
    format!("0x{mask:X}")
}
