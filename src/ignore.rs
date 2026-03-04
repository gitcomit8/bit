use std::path::PathBuf;
use std::{fs, io};

pub struct Bitignore {
    patterns: Vec<String>,
}

impl Bitignore {
    pub fn load(base_path: &PathBuf) -> io::Result<Self> {
        let ignore_path = base_path.join(".bitignore");
        if ignore_path.exists() {
            let contents = fs::read_to_string(ignore_path)?;
            let patterns = contents
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(String::from)
                .collect();
            Ok(Bitignore { patterns })
        } else {
            Ok(Bitignore {
                patterns: Vec::new(),
            })
        }
    }

    pub fn is_ignored(&self, rel_path: &str) -> bool {
        let filename = PathBuf::from(rel_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        for pattern in &self.patterns {
            if pattern.ends_with('/') {
                let dir_name = pattern.trim_end_matches('/');
                let rel = PathBuf::from(rel_path);
                for component in rel.components() {
                    if component.as_os_str() == dir_name {
                        return true;
                    }
                }
            } else if let Some(ext_pattern) = pattern.strip_prefix("*.") {
                if filename.ends_with(&format!(".{}", ext_pattern)) {
                    return true;
                }
            } else {
                if &filename == pattern || rel_path == pattern {
                    return true;
                }
            }
        }
        false
    }
}
