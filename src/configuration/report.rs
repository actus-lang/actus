//! Prints a configuration failure in the one diagnostic format.
//!
//! A failure in a manifest is `<manifest>: error[E####] at line:column: message`,
//! pointing at the key or value it is about when the manifest gives that place;
//! when it does not, it is `<manifest>: error[E####]: message`. A failure of the
//! toolchain or the environment names no file: `error[E####]: message`.

use crate::diagnostics::{
    Diagnostic, DiagnosticPhase, render_diagnostic, render_unlocated_diagnostic,
};
use crate::lexer::SourceSpan;

use super::ConfigurationError;

impl ConfigurationError {
    /// The complete line to print for this failure, without any `error: ` prefix.
    pub fn report(&self) -> String {
        let diagnostic = Diagnostic::error(self.code(), SourceSpan::new(0, 0), &self.message)
            .with_phase(DiagnosticPhase::Configuration);
        let Some(manifest) = &self.place.manifest else {
            return render_unlocated_diagnostic(&diagnostic);
        };
        let located = std::fs::read_to_string(manifest).ok().and_then(|source| {
            let offset = self
                .place
                .offset
                .or_else(|| self.place.key.and_then(|key| locate_key(&source, key)))?;
            let offset = offset.min(source.len());
            Some((source, offset))
        });
        let rendered = match located {
            Some((source, offset)) => {
                render_diagnostic(&source, &diagnostic.with_span(SourceSpan::new(offset, offset)))
            }
            None => render_unlocated_diagnostic(&diagnostic),
        };
        format!("{}: {rendered}", manifest.display())
    }
}

/// The dotted key whose key or value starts at `offset`: where the TOML
/// parser says a manifest value is wrong, which key it is.
pub(super) fn key_at(source: &str, offset: usize) -> Option<String> {
    fn find(table: &toml::de::DeTable<'_>, offset: usize, path: &mut Vec<String>) -> bool {
        for (key, value) in table.iter() {
            path.push(key.get_ref().to_string());
            if key.span().start == offset || value.span().start == offset {
                return true;
            }
            if let Some(inner) = value.get_ref().as_table()
                && find(inner, offset, path)
            {
                return true;
            }
            path.pop();
        }
        false
    }
    let document = toml::de::DeTable::parse(source).ok()?;
    let mut path = Vec::new();
    find(document.get_ref(), offset, &mut path).then(|| path.join("."))
}

/// The byte offset of the value of the dotted `key`, such as `build.runtime`.
fn locate_key(source: &str, key: &str) -> Option<usize> {
    let document = toml::de::DeTable::parse(source).ok()?;
    let mut table = document.get_ref();
    let mut parts = key.split('.').peekable();
    loop {
        let value = table.get(parts.next()?)?;
        if parts.peek().is_none() {
            return Some(value.span().start);
        }
        table = value.get_ref().as_table()?;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ConfigurationError, key_at, locate_key};
    use crate::diagnostics::{STRICT_RUNTIME_METADATA_MISSING, STRICT_RUNTIME_UNSUPPORTED};

    #[test]
    fn an_environment_failure_names_no_file_even_inside_a_manifest_load() {
        let error = ConfigurationError::environment(STRICT_RUNTIME_METADATA_MISSING, "no std")
            .in_manifest(Path::new("Actus.toml"));
        assert_eq!(error.report(), "error[E1803]: no std");
    }

    #[test]
    fn a_manifest_failure_without_a_place_names_the_file_and_no_location() {
        let error = ConfigurationError::coded(STRICT_RUNTIME_UNSUPPORTED, "no such profile")
            .at_key("build.runtime")
            .in_manifest(Path::new("does-not-exist/Actus.toml"));
        assert_eq!(error.report(), "does-not-exist/Actus.toml: error[E1804]: no such profile");
        assert_eq!(
            ConfigurationError::new("plain").code(),
            crate::diagnostics::STRICT_CONFIGURATION_FAILURE
        );
    }

    #[test]
    fn a_key_is_located_at_its_value() {
        let source = "[package]\nname = \"1app\"\n\n[build]\nruntime = \"core\"\n";
        assert_eq!(locate_key(source, "package.name"), Some(17));
        assert_eq!(locate_key(source, "build.runtime"), Some(source.find("\"core\"").unwrap()));
        assert_eq!(locate_key(source, "build.target"), None);
        assert_eq!(locate_key("[package", "package.name"), None);
    }

    #[test]
    fn the_key_at_an_offset_is_found_from_the_key_or_the_value() {
        let source = "[package]\nname = \"1app\"\n\n[build]\nruntime = \"core\"\n";
        assert_eq!(key_at(source, 17).as_deref(), Some("package.name"));
        assert_eq!(
            key_at(source, source.find("runtime").unwrap()).as_deref(),
            Some("build.runtime")
        );
        assert_eq!(key_at(source, 3), None);
    }
}
