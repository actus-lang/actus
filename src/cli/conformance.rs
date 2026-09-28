use std::path::Path;

use super::diagnostics::report_diagnostics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConformanceMode {
    Standard,
    Strict,
}

pub(super) fn validate_source_limits(path: &Path, source: &str, mode: ConformanceMode) -> bool {
    if !mode.is_strict() {
        return true;
    }
    let diagnostics = crate::conformance::inspect_source(path, source);
    if diagnostics.is_empty() {
        return true;
    }
    report_diagnostics(path, source, diagnostics);
    false
}

impl ConformanceMode {
    pub(super) const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

pub(super) fn parse_strict_option(
    argument: &str,
    mode: &mut ConformanceMode,
) -> Result<bool, String> {
    if argument != "--strict" {
        return Ok(false);
    }
    if mode.is_strict() {
        return Err("duplicate `--strict` option".to_owned());
    }
    *mode = ConformanceMode::Strict;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{ConformanceMode, parse_strict_option};

    #[test]
    fn strict_option_is_explicit_and_non_repeatable() {
        let mut mode = ConformanceMode::Standard;
        assert!(parse_strict_option("--strict", &mut mode).expect("strict option should parse"));
        assert!(mode.is_strict());
        assert_eq!(
            parse_strict_option("--strict", &mut mode),
            Err("duplicate `--strict` option".to_owned())
        );
    }

    #[test]
    fn unrelated_arguments_are_left_to_command_parsers() {
        let mut mode = ConformanceMode::Standard;
        assert!(!parse_strict_option("--release", &mut mode).expect("unrelated option"));
        assert_eq!(mode, ConformanceMode::Standard);
    }
}
