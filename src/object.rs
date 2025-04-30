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
        fs::write(object_path, &self.content);
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
}

//Represents a snapshot in time
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub parent: Option<String>, //Hash of the parent commit
    pub timestamp: i64,
    pub tree: String,        //Hash of the root tree
    pub commit_hash: String, //Hash of this commit
}

impl Commit {
    pub fn new(parent: Option<String>, timestamp: i64, tree: String, commit_hash: String) -> Self {
        Commit {
            parent,
            timestamp,
            tree,
            commit_hash,
        }
    }
}
