mod diff;
mod index;
mod object;

use crate::diff::BinaryDiff;
use clap::Parser;
use index::Index;
use libloading::{Library, Symbol};
use object::{Blob, Commit, Tree};
use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;
use std::time::SystemTime;
use std::{fs, io};

const BIT_DIR: &str = ".bit";
const PLUGIN_DIR: &str = "diff-algorithm";
const PLUGIN_CREATE_FN_NAME: &str = "create_diff_algorithm";

pub type CreateDiffAlgorithm = unsafe fn() -> Box<dyn BinaryDiff>;
#[derive(Parser, Debug)]
#[command(author,version,about,long_about=None)]
enum Cli {
    Init,
    Add {
        path: PathBuf,
    },
    Commit {
        #[arg(short,long,default_value_t=String::from("default"))]
        algorithm: String,
    },
    Log,
}

struct BitState {
    base_path: PathBuf,
    bit_path: PathBuf,
    algorithms: HashMap<String, Box<dyn BinaryDiff>>,
}

impl BitState {
    fn new(base_path: PathBuf) -> io::Result<Self> {
        let bit_path = base_path.join(BIT_DIR);
        let algorithms = load_diff_algorithms(&base_path)?;
        Ok(BitState {
            base_path,
            bit_path,
            algorithms,
        })
    }
}

fn main() -> io::Result<()> {
    let state = BitState::new(std::env::current_dir()?)?;
    let cli = Cli::parse();

    match cli {
        Cli::Init => cmd_init(&state.bit_path)?,
        Cli::Add { path } => cmd_add(&state.base_path, &state.bit_path, &path)?,
        Cli::Commit { algorithm } => cmd_commit(
            &state.base_path,
            &state.bit_path,
            &state.algorithms,
            &algorithm,
        )?,
        Cli::Log => cmd_log(&state.bit_path)?,
    }
    Ok(())
}

fn load_diff_algorithms(base_path: &PathBuf) -> io::Result<HashMap<String, Box<dyn BinaryDiff>>> {
    let mut algorithms = HashMap::new();
    let plugin_path = base_path.join(PLUGIN_DIR);

    if plugin_path.is_dir() {
        for entry in fs::read_dir(plugin_path)? {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let os_ext = std::env::consts::DLL_EXTENSION;
                    if ext == os_ext {
                        println!("Found potential plugin: {}", path.display());
                        unsafe {
                            match Library::new(&path) {
                                Ok(lib) => {
                                    match lib.get::<Symbol<CreateDiffAlgorithm>>(
                                        PLUGIN_CREATE_FN_NAME.as_bytes(),
                                    ) {
                                        Ok(create_fn) => {
                                            let algorithm = create_fn();
                                            let name = algorithm.get_name();
                                            println!("Loaded algorithm: {}", name);
                                            algorithms.insert(name.to_string(), algorithm);
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error loading symbol '{}' from {}: {}",
                                                PLUGIN_CREATE_FN_NAME,
                                                path.display(),
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error loading library {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        fs::create_dir_all(&plugin_path)?;
        println!("Created plugin directory: {}", plugin_path.display());
    }
    Ok(algorithms)
}

fn cmd_init(bit_path: &PathBuf) -> io::Result<()> {
    if !bit_path.exists() {
        fs::create_dir(bit_path)?;
        fs::create_dir(bit_path.join("objects"))?;
        println!("Initialized empty Bit repository in {}", bit_path.display());
    } else {
        println!("Bit repository already exists at {}", bit_path.display());
    }
    Ok(())
}

fn cmd_add(base_path: &PathBuf, bit_path: &PathBuf, path: &PathBuf) -> io::Result<()> {
    let content = fs::read(path)?;
    let blob = Blob::new(content);
    let hash = blob.store(bit_path)?;

    let mut index = Index::load(bit_path)?;
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    index.insert(filename, hash);
    index.save(bit_path)?;

    println!("Added {} to the index", path.display());
    Ok(())
}

fn cmd_commit(
    base_path: &PathBuf,
    bit_path: &PathBuf,
    algorithms: &HashMap<String, Box<dyn BinaryDiff>>,
    algorithm_name: &str,
) -> io::Result<()> {
    let index = Index::load(bit_path)?;
    if index.entries.is_empty() {
        println!("No files staged to commit.");
        return Ok(());
    }

    let mut tree = Tree::new();
    for (filename, hash) in &index.entries {
        tree.insert(filename.clone(), hash.clone());
    }
    let tree_hash = tree.store(bit_path)?;

    let head_path = bit_path.join("HEAD");
    let parent_hash = if head_path.exists() {
        fs::read_to_string(&head_path).ok()
    } else {
        None
    };

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let commit = Commit::new(
        parent_hash,
        timestamp as i64,
        tree_hash.clone(),
        "".to_string(),
    );
    let commit_hash = commit.calculate_hash();
    let commit_with_hash = Commit::new(
        commit.parent,
        commit.timestamp,
        commit.tree,
        commit_hash.clone(),
    );
    commit_with_hash.store(bit_path)?;

    fs::write(head_path, &commit_hash)?;
    index.save(bit_path)?; // Update index to reflect committed state

    println!("Committed with hash: {}", commit_hash);
    Ok(())
}

fn cmd_log(bit_path: &PathBuf) -> io::Result<()> {
    let head_path = bit_path.join("HEAD");
    if !head_path.exists() {
        println!("No commits yet.");
        return Ok(());
    }

    let mut current_hash = fs::read_to_string(head_path)?;

    while !current_hash.is_empty() {
        let commit = Commit::load(bit_path, &current_hash)?;
        println!("Commit: {}", commit.commit_hash);
        println!("Timestamp: {}", commit.timestamp);
        if let Some(parent) = &commit.parent {
            println!("Parent: {}", parent);
        } else {
            println!("Genesis commit");
        }
        println!();
        current_hash = commit.parent.unwrap_or_default();
    }

    Ok(())
}
