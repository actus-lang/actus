#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntrinsicKind {
    Append,
    BufferLength,
    Crc32,
    Crc32Matches,
    ValidateFixedFrame,
    Copy,
    Print,
    Drop,
    SizeOf,
    AlignOf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntrinsicSpec {
    pub name: &'static str,
    pub parameters: &'static [&'static str],
    pub status: RegistryStatus,
}

impl IntrinsicKind {
    pub const fn spec(self) -> IntrinsicSpec {
        match self {
            Self::Append => IntrinsicSpec {
                name: "append",
                parameters: &["handle", "byte"],
                status: RegistryStatus::Active,
            },
            Self::BufferLength => IntrinsicSpec {
                name: "buffer_length",
                parameters: &["buffer"],
                status: RegistryStatus::Active,
            },
            Self::Crc32 => IntrinsicSpec {
                name: "crc32",
                parameters: &["buffer", "start", "end"],
                status: RegistryStatus::Active,
            },
            Self::Crc32Matches => IntrinsicSpec {
                name: "crc32_matches",
                parameters: &["buffer", "start", "end", "expected"],
                status: RegistryStatus::Active,
            },
            Self::ValidateFixedFrame => IntrinsicSpec {
                name: "validate_fixed_frame",
                parameters: &[
                    "buffer",
                    "little",
                    "version_offset",
                    "expected_version",
                    "payload_offset",
                    "payload_length",
                    "checksum_start",
                    "checksum_end",
                    "checksum_offset",
                ],
                status: RegistryStatus::Active,
            },
            Self::Copy => IntrinsicSpec {
                name: "copy",
                parameters: &["value"],
                status: RegistryStatus::Active,
            },
            Self::Print => IntrinsicSpec {
                name: "print",
                parameters: &["value"],
                status: RegistryStatus::Active,
            },
            Self::Drop => IntrinsicSpec {
                name: "drop",
                parameters: &["binding"],
                status: RegistryStatus::Active,
            },
            Self::SizeOf => layout_spec("size_of"),
            Self::AlignOf => layout_spec("align_of"),
        }
    }
}

const fn layout_spec(name: &'static str) -> IntrinsicSpec {
    IntrinsicSpec { name, parameters: &[], status: RegistryStatus::Active }
}

pub fn lookup_intrinsic(name: &str) -> Option<IntrinsicKind> {
    match name {
        "append" => Some(IntrinsicKind::Append),
        "buffer_length" => Some(IntrinsicKind::BufferLength),
        "crc32" => Some(IntrinsicKind::Crc32),
        "crc32_matches" => Some(IntrinsicKind::Crc32Matches),
        "validate_fixed_frame" => Some(IntrinsicKind::ValidateFixedFrame),
        "copy" => Some(IntrinsicKind::Copy),
        "print" => Some(IntrinsicKind::Print),
        "drop" => Some(IntrinsicKind::Drop),
        "size_of" => Some(IntrinsicKind::SizeOf),
        "align_of" => Some(IntrinsicKind::AlignOf),
        _ => None,
    }
}

pub fn lookup_call_intrinsic(name: &str) -> Option<IntrinsicKind> {
    let base_name = name.split_once('[').map_or(name, |(base, _)| base);
    match lookup_intrinsic(base_name) {
        Some(
            IntrinsicKind::Append
            | IntrinsicKind::BufferLength
            | IntrinsicKind::Crc32
            | IntrinsicKind::Crc32Matches
            | IntrinsicKind::ValidateFixedFrame
            | IntrinsicKind::Copy
            | IntrinsicKind::Print
            | IntrinsicKind::SizeOf
            | IntrinsicKind::AlignOf,
        ) => lookup_intrinsic(base_name),
        Some(IntrinsicKind::Drop) | None => None,
    }
}
use super::RegistryStatus;
