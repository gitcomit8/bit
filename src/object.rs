use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

//Represent binary file content
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blob {
    pub content: Vec<u8>,
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Self {
        Blob { content }
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();
        hasher.update(&self.content);
        hasher.finalize().to_hex().to_string()
    }

    pub fn store(&self, base_path: &PathBuf) -> io::Result<String> {
        let hash = self.calculate_hash();
        let object_path = base_path.join("objects").join(&hash);
        fs::write(object_path, &self.content)?;
        Ok(hash)
    }

    pub fn load(base_path: &PathBuf, hash: &str) -> io::Result<Self> {
        let object_path = base_path.join("objects").join(hash);
        let content = fs::read(object_path)?;
        Ok(Blob::new(content))
    }
}

//Represents directory structure (only one level for now)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tree {
    pub entries: HashMap<String, String>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, hash: String) {
        self.entries.insert(name, hash);
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();
        // Sort entries for consistent hashing
        let mut sorted_entries: Vec<(&String, &String)> = self.entries.iter().collect();
        sorted_entries.sort_by_key(|(name, _)| *name);
        for (name, hash) in sorted_entries {
            hasher.update(name.as_bytes());
            hasher.update(hash.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    pub fn store(&self, base_path: &PathBuf) -> io::Result<String> {
        let hash = self.calculate_hash();
        let object_path = base_path.join("objects").join(&hash);
        let serialized = serde_json::to_string(self)?;
        fs::write(object_path, serialized)?;
        Ok(hash)
    }

    pub fn load(base_path: &PathBuf, hash: &str) -> io::Result<Self> {
        let object_path = base_path.join("objects").join(hash);
        let serialized = fs::read_to_string(object_path)?;
        let tree = serde_json::from_str(&serialized)?;
        Ok(tree)
    }
}

//Represents a snapshot in time
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub parent: Option<String>, //Hash of the parent commit
    pub timestamp: i64,
    pub tree: String,        //Hash of the root tree
    pub commit_hash: String, //Hash of this commit
    pub message: String,
}

impl Commit {
    pub fn new(
        parent: Option<String>,
        timestamp: i64,
        tree: String,
        commit_hash: String,
        message: String,
    ) -> Self {
        Commit {
            parent,
            timestamp,
            tree,
            commit_hash,
            message,
        }
    }
    pub fn calculate_hash(&self) -> String {
        let mut hasher = Hasher::new();
        if let Some(parent) = &self.parent {
            hasher.update(parent.as_bytes());
        }
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(self.tree.as_bytes());
        hasher.update(self.message.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    pub fn store(&self, base_path: &PathBuf) -> io::Result<String> {
        let hash = self.calculate_hash();
        let object_path = base_path.join("objects").join(&hash);
        let serialized = serde_json::to_string(self).unwrap();
        fs::write(object_path, serialized)?;
        Ok(hash)
    }

    pub fn load(base_path: &PathBuf, hash: &str) -> io::Result<Self> {
        let object_path = base_path.join("objects").join(hash);
        let serialized = fs::read_to_string(object_path)?;
        let commit = serde_json::from_str(&serialized).unwrap();
        Ok(commit)
    }
}
