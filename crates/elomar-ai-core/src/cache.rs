//! The embedding cache (`[ai.cache] embeddings = "file"` in elomar.toml):
//! vectors already received, kept in a JSON file in the project, so the
//! same text is not sent to a model again.
//!
//! An entry belongs to a model's *identity* (provider, vendor model id and
//! address), not to its name in the program: changing the model id in
//! elomar.toml makes the old vectors unused instead of wrong. Only models
//! that use the network are cached.

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::{Value, json};

pub struct EmbeddingCache {
    path: PathBuf,
    /// Logical model name → identity.
    identities: HashMap<String, String>,
    /// (identity, text) → vector.
    entries: HashMap<(String, String), Vec<f64>>,
    changed: bool,
}

impl EmbeddingCache {
    /// Reads the cache file if it exists. A missing or unreadable file is an
    /// empty cache: it is only a way to save calls.
    pub fn open(path: PathBuf) -> Self {
        let mut entries = HashMap::new();
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(json) = serde_json::from_str::<Value>(&text)
            && let Some(items) = json["entries"].as_array()
        {
            for item in items {
                let (Some(model), Some(text), Some(vector)) = (
                    item["model"].as_str(),
                    item["text"].as_str(),
                    item["vector"].as_array(),
                ) else {
                    continue;
                };
                let Some(v) = vector
                    .iter()
                    .map(Value::as_f64)
                    .collect::<Option<Vec<f64>>>()
                else {
                    continue;
                };
                if !v.is_empty() {
                    entries.insert((model.to_string(), text.to_string()), v);
                }
            }
        }
        EmbeddingCache {
            path,
            identities: HashMap::new(),
            entries,
            changed: false,
        }
    }

    /// Caches the embeddings of the model `name`, which is `identity`.
    pub fn add_model(&mut self, name: &str, identity: String) {
        self.identities.insert(name.to_string(), identity);
    }

    pub fn get(&self, model: &str, text: &str) -> Option<Vec<f64>> {
        let identity = self.identities.get(model)?;
        self.entries
            .get(&(identity.clone(), text.to_string()))
            .cloned()
    }

    pub fn put(&mut self, model: &str, text: &str, vector: &[f64]) {
        if let Some(identity) = self.identities.get(model) {
            self.entries
                .insert((identity.clone(), text.to_string()), vector.to_vec());
            self.changed = true;
        }
    }

    /// Writes the file if something was added (through a temporary file,
    /// so a stopped program never leaves half a file). Errors are ignored.
    pub fn save(&mut self) {
        if !self.changed {
            return;
        }
        self.changed = false;
        let mut items: Vec<(&(String, String), &Vec<f64>)> = self.entries.iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        let entries: Vec<Value> = items
            .into_iter()
            .map(|((model, text), v)| json!({ "model": model, "text": text, "vector": v }))
            .collect();
        let body = json!({ "version": 1, "entries": entries }).to_string();
        let tmp = self.path.with_extension("tmp");
        if std::fs::write(&tmp, body).is_ok() && std::fs::rename(&tmp, &self.path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// How many vectors are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
