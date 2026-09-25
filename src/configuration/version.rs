use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VersionConstraint {
    Exact(Version),
    Compatible(Version),
    GreaterOrEqual(Version),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionError(pub String);

impl Display for VersionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VersionError {}

impl Version {
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let parts = input.trim().split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(VersionError(format!(
                "invalid semantic version `{input}`; expected MAJOR.MINOR.PATCH"
            )));
        }
        let numbers = parts
            .iter()
            .map(|part| {
                part.parse::<u64>().map_err(|_| {
                    VersionError(format!("invalid semantic version component `{part}`"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { major: numbers[0], minor: numbers[1], patch: numbers[2] })
    }
}

impl VersionConstraint {
    pub fn parse(input: &str) -> Result<Self, VersionError> {
        let input = input.trim();
        let (operator, version) = if let Some(version) = input.strip_prefix('^') {
            ("^", version)
        } else if let Some(version) = input.strip_prefix(">=") {
            (">=", version)
        } else if let Some(version) = input.strip_prefix('=') {
            ("=", version)
        } else {
            ("=", input)
        };
        if version.is_empty() {
            return Err(VersionError(format!("missing version after `{operator}`")));
        }
        let version = Version::parse(version)?;
        Ok(match operator {
            "^" => Self::Compatible(version),
            ">=" => Self::GreaterOrEqual(version),
            _ => Self::Exact(version),
        })
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(expected) => version == expected,
            Self::GreaterOrEqual(minimum) => version >= minimum,
            Self::Compatible(minimum) => {
                version >= minimum
                    && version.major == minimum.major
                    && (minimum.major != 0 || version.minor == minimum.minor)
                    && (minimum.major != 0 || minimum.minor != 0 || version.patch == minimum.patch)
            }
        }
    }
}

pub fn validate_constraint(constraint: &str) -> Result<(), VersionError> {
    VersionConstraint::parse(constraint).map(|_| ())
}
