use std::collections::HashMap;

use super::position::{LspRange, byte_range};

#[derive(Clone, Debug)]
pub struct Document {
    pub version: i64,
    pub text: String,
}

#[derive(Default)]
pub struct DocumentStore {
    documents: HashMap<String, Document>,
}

impl DocumentStore {
    pub fn open(&mut self, uri: String, version: i64, text: String) {
        self.documents.insert(uri, Document { version, text });
    }

    pub fn close(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    pub fn change(
        &mut self,
        uri: &str,
        version: i64,
        changes: &[ContentChange],
    ) -> Result<(), String> {
        let document =
            self.documents.get_mut(uri).ok_or_else(|| format!("document `{uri}` is not open"))?;
        for change in changes {
            if let Some(range) = &change.range {
                let bytes = byte_range(&document.text, range)
                    .ok_or_else(|| "invalid document change range".to_owned())?;
                document.text.replace_range(bytes, &change.text);
            } else {
                document.text = change.text.clone();
            }
        }
        document.version = version;
        Ok(())
    }

    pub fn get(&self, uri: &str) -> Option<&Document> {
        self.documents.get(uri)
    }
}

#[derive(Clone, Debug)]
pub struct ContentChange {
    pub range: Option<LspRange>,
    pub text: String,
}
