//! The registry of repositories PocketRepo serves.
//!
//! Repositories are supplied on the command line (`pocket-repo <path>...`); each
//! path's directory name becomes its public name. The registry is read-only for
//! the lifetime of the process, so a global `OnceLock` lets `view()` reach it
//! without threading state through maudliver's pure page functions.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
}

static REGISTRY: OnceLock<BTreeMap<String, PathBuf>> = OnceLock::new();

/// Builds the registry from a list of repository paths. Later duplicates of a
/// name are silently ignored. Non-directories and non-git paths are skipped
/// with a warning so a single bad argument doesn't take the server down.
pub fn init(paths: impl IntoIterator<Item = PathBuf>) {
    let mut map = BTreeMap::new();
    for path in paths {
        let canonical = path.canonicalize().unwrap_or(path.clone());
        if !canonical.is_dir() {
            eprintln!("skipping {}: not a directory", canonical.display());
            continue;
        }
        if git2::Repository::open(&canonical).is_err() {
            eprintln!("skipping {}: not a git repository", canonical.display());
            continue;
        }
        let name = canonical
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repo".to_string());
        map.entry(name).or_insert(canonical);
    }
    let _ = REGISTRY.set(map);
}

fn registry() -> &'static BTreeMap<String, PathBuf> {
    REGISTRY.get_or_init(BTreeMap::new)
}

pub fn repos() -> Vec<RepoInfo> {
    registry()
        .iter()
        .map(|(name, path)| RepoInfo {
            name: name.clone(),
            path: path.clone(),
        })
        .collect()
}

pub fn repo_path(name: &str) -> Option<&'static Path> {
    registry().get(name).map(PathBuf::as_path)
}
