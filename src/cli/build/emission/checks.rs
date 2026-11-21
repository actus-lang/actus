//! Source locations for the runtime checks of a hosted executable (ADR-0117).

use std::collections::BTreeMap;

use crate::SharedSourceMap;
use crate::codegen::{BackendModule, RuntimeCheckReporting, runtime_check};

/// Reporting for every runtime check in `module`, each located through
/// `source_map` as `path:line:column`. Paths under the current directory are
/// shown relative to it.
pub(super) fn runtime_check_reporting(
    module: &BackendModule,
    source_map: &SharedSourceMap,
) -> RuntimeCheckReporting {
    let source_map = source_map.lock();
    let current_directory = std::env::current_dir().ok();
    let mut locations = BTreeMap::new();
    let spans = module
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| runtime_check(instruction).map(|(_, span)| span));
    for span in spans {
        let key = (span.start, span.end);
        if locations.contains_key(&key) {
            continue;
        }
        let Some((path, line, column)) = source_map.line_column(span) else { continue };
        let path = current_directory
            .as_deref()
            .and_then(|directory| path.strip_prefix(directory).ok())
            .unwrap_or(&path);
        locations.insert(key, format!("{}:{line}:{column}", path.display()));
    }
    RuntimeCheckReporting::Report { locations }
}
