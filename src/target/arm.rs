#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArmInstructionMode {
    Thumb2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm32TargetProfile {
    CortexMv7em,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Arm32AbiContract {
    pub word_bytes: u8,
    pub pointer_width_bits: u8,
    pub stack_alignment_bytes: u8,
    pub argument_registers: [u8; 4],
    pub return_registers: [u8; 2],
    pub callee_saved_registers: [u8; 8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Arm32RegisterWords {
    pub first_register: u8,
    pub word_count: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm32ArgumentLocation {
    CoreRegisters(Arm32RegisterWords),
    Stack { byte_offset: u32, byte_size: u32, alignment_bytes: u8 },
    IndirectCoreRegister { address_register: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm32ReturnLocation {
    CoreRegisters(Arm32RegisterWords),
    IndirectCoreRegister { address_register: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Arm32ValueRepresentation {
    pub size_bytes: u32,
    pub alignment_bytes: u8,
    pub argument: Arm32ArgumentLocation,
    pub return_value: Arm32ReturnLocation,
}

impl Arm32AbiContract {
    pub const fn cortex_mv7em() -> Self {
        Self {
            word_bytes: 4,
            pointer_width_bits: 32,
            stack_alignment_bytes: 8,
            argument_registers: [0, 1, 2, 3],
            return_registers: [0, 1],
            callee_saved_registers: [4, 5, 6, 7, 8, 9, 10, 11],
        }
    }

    pub const fn scalar_word_representation() -> Arm32ValueRepresentation {
        Arm32ValueRepresentation {
            size_bytes: 4,
            alignment_bytes: 4,
            argument: Arm32ArgumentLocation::CoreRegisters(Arm32RegisterWords {
                first_register: 0,
                word_count: 1,
            }),
            return_value: Arm32ReturnLocation::CoreRegisters(Arm32RegisterWords {
                first_register: 0,
                word_count: 1,
            }),
        }
    }

    pub const fn aggregate_stack_representation(
        size_bytes: u32,
        alignment_bytes: u8,
        byte_offset: u32,
    ) -> Arm32ValueRepresentation {
        Arm32ValueRepresentation {
            size_bytes,
            alignment_bytes,
            argument: Arm32ArgumentLocation::Stack {
                byte_offset,
                byte_size: size_bytes,
                alignment_bytes,
            },
            return_value: Arm32ReturnLocation::IndirectCoreRegister { address_register: 0 },
        }
    }
}
