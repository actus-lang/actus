use std::collections::BTreeMap;

use crate::ast::Program;
use crate::lexer::scan;
use crate::parser::parse;

pub(super) const MAX_PARSED_PROGRAMS: usize = 256;
const MAX_SEMANTIC_SNAPSHOTS: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ParseSnapshot {
    Valid(Program),
    Invalid,
}

#[derive(Clone, Debug)]
struct CachedProgram {
    version: i64,
    snapshot: ParseSnapshot,
    last_used: u64,
}

#[derive(Debug, Default)]
pub(super) struct QueryCache {
    parsed: BTreeMap<String, CachedProgram>,
    semantic: BTreeMap<(String, String), CachedSemantic>,
    clock: u64,
}

#[derive(Clone, Debug)]
struct CachedSemantic {
    version: i64,
    result: serde_json::Value,
}

impl QueryCache {
    pub(super) fn parse(&mut self, uri: &str, version: i64, source: &str) -> ParseSnapshot {
        self.clock = self.clock.saturating_add(1);
        if let Some(entry) = self.parsed.get_mut(uri)
            && entry.version == version
        {
            entry.last_used = self.clock;
            return entry.snapshot.clone();
        }
        let (tokens, errors) = scan(source);
        let snapshot = if errors.is_empty() {
            parse(tokens).map_or(ParseSnapshot::Invalid, ParseSnapshot::Valid)
        } else {
            ParseSnapshot::Invalid
        };
        self.parsed.insert(
            uri.to_owned(),
            CachedProgram { version, snapshot: snapshot.clone(), last_used: self.clock },
        );
        self.evict_oldest();
        snapshot
    }

    pub(super) fn invalidate(&mut self, uri: &str) {
        self.parsed.remove(uri);
        self.semantic.retain(|(cached_uri, _), _| cached_uri != uri);
    }

    pub(super) fn semantic(
        &self,
        uri: &str,
        target: &str,
        version: i64,
    ) -> Option<serde_json::Value> {
        self.semantic
            .get(&(uri.to_owned(), target.to_owned()))
            .and_then(|entry| (entry.version == version).then(|| entry.result.clone()))
    }

    pub(super) fn store_semantic(
        &mut self,
        uri: &str,
        target: &str,
        version: i64,
        result: serde_json::Value,
    ) {
        self.semantic
            .insert((uri.to_owned(), target.to_owned()), CachedSemantic { version, result });
        while self.semantic.len() > MAX_SEMANTIC_SNAPSHOTS {
            let Some(key) = self.semantic.keys().next().cloned() else { break };
            self.semantic.remove(&key);
        }
    }

    fn evict_oldest(&mut self) {
        while self.parsed.len() > MAX_PARSED_PROGRAMS {
            let Some(uri) = self
                .parsed
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(uri, _)| uri.clone())
            else {
                return;
            };
            self.parsed.remove(&uri);
        }
    }
}
