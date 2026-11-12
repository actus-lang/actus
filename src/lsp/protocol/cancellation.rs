use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde_json::Value;

#[derive(Clone, Default)]
pub(in crate::lsp) struct CancellationRegistry {
    canceled: Arc<Mutex<HashSet<String>>>,
}

#[derive(Clone)]
pub(in crate::lsp) struct CancellationToken {
    key: String,
    registry: CancellationRegistry,
}

impl CancellationRegistry {
    pub(in crate::lsp) fn token(&self, id: Option<&Value>) -> Option<CancellationToken> {
        id.map(|id| CancellationToken { key: id.to_string(), registry: self.clone() })
    }

    pub(in crate::lsp) fn cancel(&self, id: &Value) {
        if let Ok(mut canceled) = self.canceled.lock() {
            canceled.insert(id.to_string());
        }
    }

    fn clear(&self, key: &str) {
        if let Ok(mut canceled) = self.canceled.lock() {
            canceled.remove(key);
        }
    }
}

impl CancellationToken {
    pub(in crate::lsp) fn is_canceled(&self) -> bool {
        self.registry.canceled.lock().map(|canceled| canceled.contains(&self.key)).unwrap_or(true)
    }

    pub(in crate::lsp) fn checkpoint(&self) -> bool {
        self.is_canceled()
    }
}

impl Drop for CancellationToken {
    fn drop(&mut self) {
        self.registry.clear(&self.key);
    }
}
