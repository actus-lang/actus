use super::signature::CAbiError;

impl std::fmt::Display for CAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReturnType { .. }
            | Self::MissingUnsafeBoundary { .. }
            | Self::InvalidOwnership { .. }
            | Self::InvalidReturnOwnership { .. }
            | Self::UnsupportedType { .. }
            | Self::AggregateRequiresReprC { .. } => self.fmt_primary(formatter),
            Self::TransparentCombinedRepresentation { .. }
            | Self::TransparentRequiresOneField { .. }
            | Self::TransparentFieldRole { .. }
            | Self::TransparentFieldType { .. } => self.fmt_transparent(formatter),
            Self::AggregateFieldRole { .. }
            | Self::AggregateFieldType { .. }
            | Self::AggregateLayoutOverflow { .. }
            | Self::GenericAggregate { .. }
            | Self::TargetDependentType { .. } => self.fmt_aggregate(formatter),
        }
    }
}

impl CAbiError {
    fn fmt_primary(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReturnType { verb } => {
                write!(formatter, "C ABI export `{verb}` requires an explicit return type")
            }
            Self::MissingUnsafeBoundary { verb } => {
                write!(formatter, "external C verb `{verb}` requires an explicit `unsafe` boundary")
            }
            Self::InvalidOwnership { parameter, role, ty } => write!(
                formatter,
                "C ABI parameter `{parameter}` cannot use role `{role}` with type `{ty}"
            ),
            Self::InvalidReturnOwnership { access, ty } => {
                write!(formatter, "C ABI return cannot use access `{access}` with type `{ty}")
            }
            Self::UnsupportedType { name } => {
                write!(formatter, "type `{name}` has no C ABI mapping")
            }
            Self::AggregateRequiresReprC { name } => {
                write!(formatter, "aggregate `{name}` requires `meta repr(C)` for C ABI use")
            }
            _ => unreachable!("primary C ABI error formatter received another variant"),
        }
    }

    fn fmt_transparent(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransparentCombinedRepresentation { name } => write!(
                formatter,
                "transparent aggregate `{name}` cannot combine C or packed representation"
            ),
            Self::TransparentRequiresOneField { name } => {
                write!(formatter, "transparent aggregate `{name}` requires exactly one field")
            }
            Self::TransparentFieldRole { aggregate, field, role } => write!(
                formatter,
                "transparent aggregate `{aggregate}` field `{field}` cannot use role `{role}`"
            ),
            Self::TransparentFieldType { aggregate, field, ty } => write!(
                formatter,
                "transparent aggregate `{aggregate}` field `{field}` has unsupported type `{ty}`"
            ),
            _ => unreachable!("transparent C ABI error formatter received another variant"),
        }
    }

    fn fmt_aggregate(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AggregateFieldRole { aggregate, field, role } => write!(
                formatter,
                "C ABI aggregate `{aggregate}` field `{field}` cannot use role `{role}`"
            ),
            Self::AggregateFieldType { aggregate, field, ty } => write!(
                formatter,
                "C ABI aggregate `{aggregate}` field `{field}` has unsupported type `{ty}`"
            ),
            Self::AggregateLayoutOverflow { name } => {
                write!(formatter, "C ABI aggregate `{name}` layout exceeds the supported size")
            }
            Self::GenericAggregate { name } => {
                write!(formatter, "generic aggregate `{name}` cannot cross a C ABI")
            }
            Self::TargetDependentType { name } => {
                write!(formatter, "type `{name}` requires an explicit C ABI target")
            }
            _ => unreachable!("aggregate C ABI error formatter received another variant"),
        }
    }
}

impl std::error::Error for CAbiError {}
