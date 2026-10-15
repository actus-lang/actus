use super::super::contract::{BackendContractError, BackendLayout};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendAggregateLocation {
    Register { index: u16 },
    Stack { offset: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendAggregateFragment {
    pub offset: u32,
    pub size: u32,
    pub alignment: u32,
    pub location: BackendAggregateLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDirectAggregateAbi {
    pub layout: BackendLayout,
    pub parameter_fragments: Vec<BackendAggregateFragment>,
    pub return_fragments: Vec<BackendAggregateFragment>,
}

impl BackendDirectAggregateAbi {
    pub fn validate(&self) -> Result<(), BackendContractError> {
        self.layout.validate()?;
        validate_fragments(self.layout, &self.parameter_fragments)?;
        validate_fragments(self.layout, &self.return_fragments)
    }
}

fn validate_fragments(
    layout: BackendLayout,
    fragments: &[BackendAggregateFragment],
) -> Result<(), BackendContractError> {
    if fragments.is_empty() {
        return Err(BackendContractError::InvalidAggregateAbi(
            "direct aggregate ABI requires at least one fragment",
        ));
    }
    let mut expected_offset = 0u32;
    let mut register_indices = Vec::new();
    let mut stack_ranges = Vec::new();
    for fragment in fragments {
        if fragment.size == 0
            || fragment.alignment == 0
            || !fragment.alignment.is_power_of_two()
            || fragment.size % fragment.alignment != 0
            || fragment.offset != expected_offset
            || fragment.offset % fragment.alignment != 0
            || fragment.offset.saturating_add(fragment.size) > layout.size
        {
            return Err(BackendContractError::InvalidAggregateAbi(
                "direct aggregate fragments must cover aligned contiguous layout ranges",
            ));
        }
        match fragment.location {
            BackendAggregateLocation::Register { index } => {
                if register_indices.contains(&index) {
                    return Err(BackendContractError::InvalidAggregateAbi(
                        "direct aggregate register fragments must use unique indices",
                    ));
                }
                register_indices.push(index);
            }
            BackendAggregateLocation::Stack { offset } => {
                if offset % fragment.alignment != 0 {
                    return Err(BackendContractError::InvalidAggregateAbi(
                        "direct aggregate stack fragments must be aligned",
                    ));
                }
                let end = offset.saturating_add(fragment.size);
                if stack_ranges.iter().any(|(start, stop)| offset < *stop && *start < end) {
                    return Err(BackendContractError::InvalidAggregateAbi(
                        "direct aggregate stack fragments must not overlap",
                    ));
                }
                stack_ranges.push((offset, end));
            }
        }
        expected_offset = fragment.offset + fragment.size;
    }
    (expected_offset == layout.size).then_some(()).ok_or(BackendContractError::InvalidAggregateAbi(
        "direct aggregate fragments must cover the complete layout",
    ))
}
