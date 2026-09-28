use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceExceptionManifest {
    version: u32,
    #[serde(default)]
    exceptions: Vec<SourceException>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceException {
    path: String,
    category: String,
    owner: String,
    scope: String,
    reason: String,
    replacement_plan: String,
}

/// Validates the reviewed source-exception manifest contract.
pub fn validate_source_exception_manifest(source: &str) -> Result<(), String> {
    let manifest: SourceExceptionManifest = toml::from_str(source)
        .map_err(|error| format!("invalid source-exception manifest: {error}"))?;
    if manifest.version != 1 {
        return Err(format!("unsupported source-exception manifest version {}", manifest.version));
    }
    for exception in manifest.exceptions {
        validate_exception(&exception)?;
    }
    Ok(())
}

/// Loads and validates the repository's source-exception manifest.
pub fn validate_repository_source_exceptions(root: &Path) -> Result<(), String> {
    let path = root.join("docs/conformance/source-exceptions.toml");
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
    validate_source_exception_manifest(&source)
}

fn validate_exception(exception: &SourceException) -> Result<(), String> {
    require_text("path", &exception.path)?;
    require_text("owner", &exception.owner)?;
    require_text("scope", &exception.scope)?;
    require_text("reason", &exception.reason)?;
    require_text("replacement_plan", &exception.replacement_plan)?;
    if !matches!(exception.category.as_str(), "generated" | "tabular") {
        return Err(format!(
            "source exception `{}` must use category `generated` or `tabular`",
            exception.path
        ));
    }
    let path = PathBuf::from(&exception.path);
    if path.is_absolute()
        || path.components().any(|component| component == std::path::Component::ParentDir)
    {
        return Err(format!(
            "source exception path `{}` must be repository-relative",
            exception.path
        ));
    }
    Ok(())
}

fn require_text(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("source exception field `{field}` must not be empty"));
    }
    Ok(())
}
