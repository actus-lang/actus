use crate::cli::terminal_output::{errln, outln};
use std::fs;
use std::path::{Path, PathBuf};

use crate::configuration::{CompilerConfiguration, package_identity};
use crate::packaging::create_package_archive;
use crate::registry::{LocalRegistry, PackageCache, PackageRecord, TrustPolicy};

struct PublishOptions {
    input: PathBuf,
    registry: Option<PathBuf>,
    cache: Option<PathBuf>,
}

pub(super) fn publish_command(arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            errln!("error: {error}");
            return 2;
        }
    };
    let manifest = manifest_path(&options.input);
    let configuration = match CompilerConfiguration::from_manifest(&manifest) {
        Ok(configuration) => configuration,
        Err(error) => {
            errln!("{}", error.report());
            return 1;
        }
    };
    match publish_project(&manifest, &configuration, &options) {
        Ok(report) => {
            outln!("published {}@{}", report.name, report.version);
            outln!("checksum: {}", report.checksum);
            outln!("registry archive: {}", report.destination.display());
            0
        }
        Err(error) => {
            errln!("error: {error}");
            1
        }
    }
}

struct PublishReport {
    name: String,
    version: String,
    checksum: String,
    destination: PathBuf,
}

fn publish_project(
    manifest: &Path,
    configuration: &CompilerConfiguration,
    options: &PublishOptions,
) -> Result<PublishReport, String> {
    let identity = package_identity(manifest).map_err(|error| error.to_string())?;
    let output = temporary_archive(&identity.name);
    let result =
        publish_archive(&output, &identity.name, &identity.version, configuration, options);
    remove_temporary_archive(&output);
    result
}

fn publish_archive(
    output: &Path,
    name: &str,
    version: &str,
    configuration: &CompilerConfiguration,
    options: &PublishOptions,
) -> Result<PublishReport, String> {
    let archive = create_package_archive(configuration.project_root(), output)
        .map_err(|error| format!("cannot package project: {error}"))?;
    let record = PackageRecord {
        name: name.to_owned(),
        version: version.to_owned(),
        checksum: archive.checksum.clone(),
        archive: format!("{name}-{version}.actus"),
    };
    let registry_path = options
        .registry
        .clone()
        .unwrap_or_else(|| configuration.project_root().join(".actus/registry"));
    let registry = LocalRegistry::open(&registry_path);
    let destination = registry
        .publish(output, record, &TrustPolicy::integrity_only())
        .map_err(|error| format!("cannot publish package: {error}"))?;
    let cache_path =
        options.cache.clone().unwrap_or_else(|| configuration.project_root().join(".actus/cache"));
    PackageCache::new(cache_path)
        .store(output, &archive.checksum)
        .map_err(|error| format!("cannot update package cache: {error}"))?;
    Ok(PublishReport {
        name: name.to_owned(),
        version: version.to_owned(),
        checksum: archive.checksum,
        destination,
    })
}

fn parse_options(mut arguments: impl Iterator<Item = String>) -> Result<PublishOptions, String> {
    let mut input = PathBuf::from("Actus.toml");
    let mut registry = None;
    let mut cache = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--registry" => registry = Some(next_path(&mut arguments, "--registry")?),
            "--cache" => cache = Some(next_path(&mut arguments, "--cache")?),
            _ if input == Path::new("Actus.toml") && !super::input::is_option_like(&argument) => {
                input = PathBuf::from(argument);
            }
            _ => return Err(super::input::argument_error("publish", &argument)),
        }
    }
    Ok(PublishOptions { input, registry, cache })
}

fn next_path(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<PathBuf, String> {
    arguments.next().map(PathBuf::from).ok_or_else(|| format!("missing path after `{option}`"))
}

fn manifest_path(input: &Path) -> PathBuf {
    let input = if input.is_absolute() {
        input.to_owned()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(input)
    };
    if input.is_dir() {
        crate::configuration::manifest_path_in_directory(&input)
            .unwrap_or_else(|| input.join("Actus.toml"))
    } else if input.file_name().and_then(|name| name.to_str()) == Some("Actus.toml") {
        input
    } else {
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        crate::configuration::manifest_path_in_directory(parent)
            .unwrap_or_else(|| parent.join("Actus.toml"))
    }
}

fn temporary_archive(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("actus-publish-{name}-{}.actus", std::process::id()))
}

fn remove_temporary_archive(path: &Path) {
    let _ = fs::remove_file(path);
}
