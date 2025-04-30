use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Index {
    pub entries: HashMap<String, String>, //filename -> blob hash of the latest version
}

impl Index {
    pub fn new() -> Index {
        Index::default()
    }

    pub fn insert(&mut self, filename: String, hash: String) {
        self.entries.insert(filename, hash);
    }

    pub fn get(&self, filename: &str) -> Option<&String> {
        self.entries.get(filename)
    }

    pub fn load(base_path: &PathBuf) -> io::Result<Self> {
        let index_path = base_path.join("index");
        if index_path.exists() {
            let serialized = fs::read_to_string(index_path)?;
            let index = serde_json::from_str(&serialized)?;
            Ok(index)
        } else {
            Ok(Index::new())
        }
    }

    pub fn save(&self, base_path: &PathBuf) -> io::Result<()> {
        let index_path = base_path.join("index");
        let serialized = serde_json::to_string(self)?;
        fs::write(index_path, serialized)?;
        Ok(())
    }
}
