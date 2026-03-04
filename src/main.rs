mod diff;
mod ignore;
mod index;
mod object;

use crate::diff::BinaryDiff;
use crate::ignore::Bitignore;
use clap::Parser;
use index::Index;
use libloading::{Library, Symbol};
use object::{Blob, Commit, Tree};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs, io};
use walkdir::WalkDir;

const BIT_DIR: &str = ".bit";
const PLUGIN_DIR: &str = "diff-algorithm";
const PLUGIN_CREATE_FN_NAME: &str = "create_diff_algorithm";

pub type CreateDiffAlgorithm = unsafe fn() -> Box<dyn BinaryDiff>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Cli {
    Init,
    Add {
        path: PathBuf,
    },
    Rm {
        path: PathBuf,
    },
    Commit {
        #[arg(short, long, default_value_t = String::from("default"))]
        algorithm: String,
        #[arg(short = 'm', long, default_value_t = String::new())]
        message: String,
    },
    Log,
    Status,
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
        Cli::Rm { path } => cmd_rm(&state.bit_path, &path)?,
        Cli::Commit { algorithm, message } => cmd_commit(
            &state.base_path,
            &state.bit_path,
            &state.algorithms,
            &algorithm,
            &message,
        )?,
        Cli::Log => cmd_log(&state.bit_path)?,
        Cli::Status => cmd_status(&state.base_path, &state.bit_path)?,
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

fn add_file(
    base_path: &Path,
    bit_path: &PathBuf,
    path: &Path,
    index: &mut Index,
) -> io::Result<()> {
    let content = fs::read(path)?;
    let blob = Blob::new(content);
    let hash = blob.store(bit_path)?;

    let rel_path = path
        .strip_prefix(base_path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    index.insert(rel_path, hash);
    println!("Added {} to the index", path.display());
    Ok(())
}

fn cmd_add(base_path: &PathBuf, bit_path: &PathBuf, path: &PathBuf) -> io::Result<()> {
    let mut index = Index::load(bit_path)?;

    if path.is_dir() {
        let ignore = Bitignore::load(base_path)?;
        let bit_dir = base_path.join(BIT_DIR);
        let mut count = 0usize;

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            if file_path.starts_with(&bit_dir) {
                continue;
            }
            let rel = file_path
                .strip_prefix(base_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();
            if ignore.is_ignored(&rel) {
                continue;
            }
            add_file(base_path, bit_path, file_path, &mut index)?;
            count += 1;
        }
        println!("Added {} file(s) from directory {}", count, path.display());
    } else {
        add_file(base_path, bit_path, path, &mut index)?;
    }

    index.save(bit_path)?;
    Ok(())
}

fn cmd_rm(bit_path: &PathBuf, path: &PathBuf) -> io::Result<()> {
    let mut index = Index::load(bit_path)?;
    let key = path.to_string_lossy().to_string();
    if index.remove(&key) {
        index.save(bit_path)?;
        println!("Removed {} from the index", path.display());
    } else {
        eprintln!("'{}' is not in the index", path.display());
    }
    Ok(())
}

fn cmd_commit(
    _base_path: &PathBuf,
    bit_path: &PathBuf,
    _algorithms: &HashMap<String, Box<dyn BinaryDiff>>,
    _algorithm_name: &str,
    message: &str,
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
        message.to_string(),
    );
    let commit_hash = commit.calculate_hash();
    let commit_with_hash = Commit::new(
        commit.parent,
        commit.timestamp,
        commit.tree,
        commit_hash.clone(),
        commit.message,
    );
    commit_with_hash.store(bit_path)?;

    fs::write(head_path, &commit_hash)?;
    index.save(bit_path)?;

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
        if !commit.message.is_empty() {
            println!("Message: {}", commit.message);
        }
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

fn cmd_status(base_path: &PathBuf, bit_path: &PathBuf) -> io::Result<()> {
    let index = Index::load(bit_path)?;
    let ignore = Bitignore::load(base_path)?;
    let bit_dir = base_path.join(BIT_DIR);

    let head_tree: HashMap<String, String> = {
        let head_path = bit_path.join("HEAD");
        if head_path.exists() {
            let commit_hash = fs::read_to_string(&head_path)?;
            if !commit_hash.is_empty() {
                let commit = Commit::load(bit_path, &commit_hash)?;
                let tree = Tree::load(bit_path, &commit.tree)?;
                tree.entries
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        }
    };

    let mut staged: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();

    for (rel_path, index_hash) in &index.entries {
        let full_path = base_path.join(rel_path);
        if full_path.exists() {
            let content = fs::read(&full_path)?;
            let current_hash = Blob::new(content).calculate_hash();
            if &current_hash == index_hash {
                staged.push(rel_path.clone());
            } else {
                modified.push(rel_path.clone());
            }
        } else {
            deleted.push(rel_path.clone());
        }
    }

    let mut untracked: Vec<String> = Vec::new();
    for entry in WalkDir::new(base_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        if file_path.starts_with(&bit_dir) {
            continue;
        }
        let rel = file_path
            .strip_prefix(base_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
        if ignore.is_ignored(&rel) {
            continue;
        }
        if !index.entries.contains_key(&rel) && !head_tree.contains_key(&rel) {
            untracked.push(rel);
        }
    }

    staged.sort();
    modified.sort();
    deleted.sort();
    untracked.sort();

    println!("On branch main");
    println!();

    if staged.is_empty() && modified.is_empty() && deleted.is_empty() {
        println!("Nothing to commit, working tree clean");
    } else {
        if !staged.is_empty() || !modified.is_empty() || !deleted.is_empty() {
            println!("Changes to be committed:");
            for f in &staged {
                println!("        staged:   {}", f);
            }
            for f in &modified {
                println!("        modified: {}", f);
            }
            for f in &deleted {
                println!("        deleted:  {}", f);
            }
            println!();
        }
    }

    if !untracked.is_empty() {
        println!("Untracked files:");
        for f in &untracked {
            println!("        {}", f);
        }
        println!();
    }

    Ok(())
}
