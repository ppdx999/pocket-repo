//! Thin read-only wrapper over `git2` for the operations PocketRepo needs:
//! listing a tree at a path and reading a blob at a path (from `HEAD`).

use std::path::Path;

use git2::{ObjectType, Repository, Tree};

use crate::config;

/// A lightweight error type so pages can render a message instead of panicking.
#[derive(Debug)]
pub enum GitError {
    RepoNotFound(String),
    PathNotFound(String),
    Git(git2::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::RepoNotFound(name) => write!(f, "repository not found: {name}"),
            GitError::PathNotFound(path) => write!(f, "path not found: {path}"),
            GitError::Git(e) => write!(f, "git error: {}", e.message()),
        }
    }
}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::Git(e)
    }
}

pub type Result<T> = std::result::Result<T, GitError>;

/// One entry in a directory listing.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub is_dir: bool,
}

/// The result of resolving a path inside a repository.
pub enum Resolved {
    Dir(Vec<TreeEntry>),
    File(Blob),
}

pub struct Blob {
    pub content: Vec<u8>,
    pub is_binary: bool,
}

fn open(repo_name: &str) -> Result<Repository> {
    let path = config::repo_path(repo_name)
        .ok_or_else(|| GitError::RepoNotFound(repo_name.to_string()))?;
    Ok(Repository::open(path)?)
}

fn head_tree(repo: &Repository) -> Result<Tree<'_>> {
    Ok(repo.head()?.peel_to_tree()?)
}

/// Resolves `path` (relative to the repo root, `""` for root) against `HEAD`,
/// returning either a directory listing or a file blob.
pub fn resolve(repo_name: &str, path: &str) -> Result<Resolved> {
    let repo = open(repo_name)?;
    let root = head_tree(&repo)?;

    let path = path.trim_matches('/');
    let object = if path.is_empty() {
        root.as_object().clone()
    } else {
        root.get_path(Path::new(path))
            .map_err(|_| GitError::PathNotFound(path.to_string()))?
            .to_object(&repo)?
    };

    match object.kind() {
        Some(ObjectType::Tree) => {
            let tree = object.peel_to_tree()?;
            Ok(Resolved::Dir(list_tree(&tree)))
        }
        Some(ObjectType::Blob) => {
            let blob = object.peel_to_blob()?;
            Ok(Resolved::File(Blob {
                content: blob.content().to_vec(),
                is_binary: blob.is_binary(),
            }))
        }
        _ => Err(GitError::PathNotFound(path.to_string())),
    }
}

fn list_tree(tree: &Tree) -> Vec<TreeEntry> {
    let mut entries: Vec<TreeEntry> = tree
        .iter()
        .filter_map(|entry| {
            let name = entry.name().ok()?.to_string();
            let is_dir = entry.kind() == Some(ObjectType::Tree);
            Some(TreeEntry { name, is_dir })
        })
        .collect();

    // Directories first, then files, each alphabetically (case-insensitive).
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}
