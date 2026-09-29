use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use super::position::{LspRange, byte_range};
use super::workspace::{MAX_DOCUMENT_BYTES, MAX_OPEN_DOCUMENTS, WorkspaceModel};
use crate::target::TargetSpec;

#[derive(Clone, Debug)]
pub struct Document {
    pub version: i64,
    pub text: String,
}

#[derive(Default)]
pub struct DocumentStore {
    documents: BTreeMap<String, Document>,
    workspace: WorkspaceModel,
}

impl DocumentStore {
    pub fn open(&mut self, uri: String, version: i64, text: String) -> Result<(), String> {
        if text.len() > MAX_DOCUMENT_BYTES {
            return Err(format!("document exceeds {MAX_DOCUMENT_BYTES} byte limit"));
        }
        let old_length = self.documents.get(&uri).map(|document| document.text.len());
        if let Some(old_length) = old_length {
            let current_version = self.documents.get(&uri).expect("document exists").version;
            if version <= current_version {
                return Err(format!(
                    "stale document version for `{uri}`: current {current_version}, received {version}"
                ));
            }
            self.workspace.replace(&uri, old_length, text.len())?;
        } else {
            if self.documents.len() >= MAX_OPEN_DOCUMENTS {
                return Err(format!("workspace has more than {MAX_OPEN_DOCUMENTS} open documents"));
            }
            self.workspace.open(&uri, self.documents.len() + 1, &text)?;
        }
        self.documents.insert(uri, Document { version, text });
        self.workspace.refresh(&self.documents);
        Ok(())
    }

    pub fn close(&mut self, uri: &str) {
        if let Some(document) = self.documents.remove(uri) {
            self.workspace.close(uri, document.text.len());
            self.workspace.refresh(&self.documents);
        }
    }

    pub fn change(
        &mut self,
        uri: &str,
        version: i64,
        changes: &[ContentChange],
    ) -> Result<(), String> {
        let document =
            self.documents.get(uri).ok_or_else(|| format!("document `{uri}` is not open"))?;
        if version <= document.version {
            return Err(format!(
                "stale document version for `{uri}`: current {}, received {version}",
                document.version
            ));
        }
        let mut updated_text = document.text.clone();
        for change in changes {
            if let Some(range) = &change.range {
                let bytes = byte_range(&updated_text, range)
                    .ok_or_else(|| "invalid document change range".to_owned())?;
                updated_text.replace_range(bytes, &change.text);
            } else {
                updated_text = change.text.clone();
            }
        }
        self.workspace.replace(uri, document.text.len(), updated_text.len())?;
        let document = self.documents.get_mut(uri).expect("document checked above");
        document.text = updated_text;
        document.version = version;
        self.workspace.refresh(&self.documents);
        Ok(())
    }

    pub fn get(&self, uri: &str) -> Option<&Document> {
        self.documents.get(uri)
    }

    pub(super) fn documents_snapshot(&self) -> Vec<(String, Document)> {
        self.documents.iter().map(|(uri, document)| (uri.clone(), document.clone())).collect()
    }

    pub fn set_target(&mut self, target: &TargetSpec) {
        self.workspace.set_target(&target.triple().to_string());
    }

    pub fn workspace_generation(&self) -> u64 {
        self.workspace.generation()
    }

    pub fn source_overlays(&self) -> HashMap<PathBuf, String> {
        self.workspace.overlays(&self.documents)
    }
}

#[derive(Clone, Debug)]
pub struct ContentChange {
    pub range: Option<LspRange>,
    pub text: String,
}
