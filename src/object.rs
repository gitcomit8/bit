use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//Represent binary file content
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Blob{
    pub content: Vec<u8>,
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Self {
        Blob { content}
    }
}

//Represents directory structure (only one level for now)
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Tree{
    pub entries: HashMap<String,String>,
}

impl Tree {
    pub fn new() -> Self {
        Tree {entries: HashMap::new()}
    }
    
    pub fn insert(&mut self, name: String, hash: String){
        self.entries.insert(name,hash);
    }
}

//Represents a snapshot in time
#[derive(Serialize, Deserialize, Debug,Clone)]
pub struct Commit{
    pub parent: Option<String>, //Hash of the parent commit
    pub timestamp: i64,
    pub tree: String,           //Hash of the root tree
    pub commit_hash: String,    //Hash of this commit
}

impl Commit{
    pub fn new(parent: Option<String>, timestamp: i64, tree: String, commit_hash: String) -> Self {
        Commit{parent,timestamp,tree,commit_hash,}
    }
}