//! Thin read-only wrapper over `git2` for the operations PocketRepo needs:
//! listing a tree at a path, reading a blob, and computing diffs (from `HEAD`,
//! commit history, or the working tree).

use std::path::Path;

use git2::{
    BranchType, Delta, DiffFlags, DiffOptions, ObjectType, Patch, Repository, Tree, TreeWalkMode,
    TreeWalkResult,
};

use crate::config;

/// A lightweight error type so pages can render a message instead of panicking.
#[derive(Debug)]
pub enum GitError {
    RepoNotFound(String),
    RefNotFound(String),
    PathNotFound(String),
    Git(git2::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::RepoNotFound(name) => write!(f, "repository not found: {name}"),
            GitError::RefNotFound(r) => write!(f, "ref not found: {r}"),
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

/// The tree for a ref (branch/tag/oid), or `HEAD` when `ref_name` is `None`.
fn tree_for_ref<'a>(repo: &'a Repository, ref_name: Option<&str>) -> Result<Tree<'a>> {
    match ref_name {
        None => head_tree(repo),
        Some(r) => Ok(repo
            .revparse_single(r)
            .map_err(|_| GitError::RefNotFound(r.to_string()))?
            .peel_to_tree()?),
    }
}

/// The current branch name (`HEAD` shorthand), if on a branch.
pub fn head_branch(repo_name: &str) -> Option<String> {
    let repo = open(repo_name).ok()?;
    let head = repo.head().ok()?;
    head.shorthand().ok().map(String::from)
}

/// A branch or tag, for the refs picker.
#[derive(Debug, Clone)]
pub struct RefInfo {
    pub name: String,
    pub is_tag: bool,
    pub is_head: bool,
}

/// Local branches (current first) followed by tags.
pub fn list_refs(repo_name: &str) -> Result<Vec<RefInfo>> {
    let repo = open(repo_name)?;

    let mut branches: Vec<RefInfo> = Vec::new();
    for entry in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = entry?;
        let is_head = branch.is_head();
        if let Some(name) = branch.name()?.map(String::from) {
            branches.push(RefInfo {
                name,
                is_tag: false,
                is_head,
            });
        }
    }
    branches.sort_by(|a, b| {
        b.is_head
            .cmp(&a.is_head)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let mut tags: Vec<RefInfo> = repo
        .tag_names(None)?
        .iter()
        .filter_map(|r| r.ok().flatten())
        .map(|name| RefInfo {
            name: name.to_string(),
            is_tag: true,
            is_head: false,
        })
        .collect();
    tags.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    branches.extend(tags);
    Ok(branches)
}

/// Resolves `path` (relative to the repo root, `""` for root) against `ref_name`
/// (or `HEAD`), returning either a directory listing or a file blob.
pub fn resolve(repo_name: &str, ref_name: Option<&str>, path: &str) -> Result<Resolved> {
    let repo = open(repo_name)?;
    let root = tree_for_ref(&repo, ref_name)?;

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

/// Every file path in `ref_name` (or `HEAD`), recursively (repo-root-relative).
pub fn list_files(repo_name: &str, ref_name: Option<&str>) -> Result<Vec<String>> {
    let repo = open(repo_name)?;
    let tree = tree_for_ref(&repo, ref_name)?;

    let mut files = Vec::new();
    tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(ObjectType::Blob) {
            if let Ok(name) = entry.name() {
                // `dir` is the parent path with a trailing slash (or "" at root).
                files.push(format!("{dir}{name}"));
            }
        }
        TreeWalkResult::Ok
    })?;

    Ok(files)
}

// ---------------------------------------------------------------------------
// Commit history + diffs
// ---------------------------------------------------------------------------

/// One commit in the first-parent history, for the diff timeline.
#[derive(Debug, Clone)]
pub struct RevInfo {
    pub oid: String,
    pub short: String,
    pub summary: String,
    pub author: String,
    pub when: String,
}

/// The first-parent chain starting at `HEAD` (newest first), up to `limit`
/// commits. Empty if the repo has no commits yet.
pub fn history(repo_name: &str, limit: usize) -> Result<Vec<RevInfo>> {
    let repo = open(repo_name)?;
    let mut commit = match repo.head().and_then(|h| h.peel_to_commit()) {
        Ok(c) => c,
        Err(_) => return Ok(vec![]),
    };

    let mut out = Vec::new();
    for _ in 0..limit {
        let oid = commit.id().to_string();
        out.push(RevInfo {
            short: oid[..7.min(oid.len())].to_string(),
            oid,
            summary: commit.summary().ok().flatten().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("").to_string(),
            when: format_time(commit.time().seconds()),
        });
        match commit.parent(0) {
            Ok(parent) => commit = parent,
            Err(_) => break,
        }
    }
    Ok(out)
}

/// What to diff.
pub enum DiffTarget {
    /// Uncommitted changes (index + working tree) against `HEAD`.
    WorkingTree,
    /// A commit against its first parent (or the empty tree for a root commit).
    Commit(String),
}

pub struct DiffLineRow {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

pub struct DiffHunkBlock {
    pub header: String,
    pub lines: Vec<DiffLineRow>,
}

pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: char,
    pub is_binary: bool,
    pub additions: usize,
    pub deletions: usize,
    pub hunks: Vec<DiffHunkBlock>,
}

pub struct DiffView {
    pub files: Vec<FileDiff>,
}

pub fn compute_diff(repo_name: &str, target: &DiffTarget) -> Result<DiffView> {
    let repo = open(repo_name)?;
    let mut opts = DiffOptions::new();
    opts.context_lines(3);

    let diff = match target {
        DiffTarget::WorkingTree => {
            // Include new (untracked) files — they're the point of reviewing
            // uncommitted work — but not ignored build artifacts.
            opts.include_untracked(true).recurse_untracked_dirs(true);
            let head = head_tree(&repo).ok();
            repo.diff_tree_to_workdir_with_index(head.as_ref(), Some(&mut opts))?
        }
        DiffTarget::Commit(hex) => {
            // `revparse_single` resolves both full and abbreviated oids (unlike
            // `Oid::from_str`, which zero-pads short hex into a bogus oid).
            let commit = repo
                .revparse_single(hex)
                .map_err(|_| GitError::PathNotFound(hex.clone()))?
                .peel_to_commit()?;
            let tree = commit.tree()?;
            let parent_tree = match commit.parent(0) {
                Ok(parent) => Some(parent.tree()?),
                Err(_) => None,
            };
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?
        }
    };

    build_view(&diff)
}

fn build_view(diff: &git2::Diff) -> Result<DiffView> {
    let mut files = Vec::new();

    for i in 0..diff.deltas().len() {
        let delta = match diff.get_delta(i) {
            Some(d) => d,
            None => continue,
        };

        let mut file = FileDiff {
            old_path: path_string(delta.old_file().path()),
            new_path: path_string(delta.new_file().path()),
            status: status_char(delta.status()),
            is_binary: delta.flags().contains(DiffFlags::BINARY),
            additions: 0,
            deletions: 0,
            hunks: Vec::new(),
        };

        if let Some(patch) = Patch::from_diff(diff, i)? {
            for h in 0..patch.num_hunks() {
                let (hunk, line_count) = patch.hunk(h)?;
                let mut lines = Vec::new();
                for l in 0..line_count {
                    let line = patch.line_in_hunk(h, l)?;
                    let origin = line.origin();
                    match origin {
                        '+' => file.additions += 1,
                        '-' => file.deletions += 1,
                        _ => {}
                    }
                    let mut content = String::from_utf8_lossy(line.content()).into_owned();
                    if content.ends_with('\n') {
                        content.pop();
                        if content.ends_with('\r') {
                            content.pop();
                        }
                    }
                    lines.push(DiffLineRow {
                        origin,
                        content,
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }
                file.hunks.push(DiffHunkBlock {
                    header: String::from_utf8_lossy(hunk.header()).trim_end().to_string(),
                    lines,
                });
            }
        }

        files.push(file);
    }

    Ok(DiffView { files })
}

fn path_string(path: Option<&Path>) -> Option<String> {
    path.map(|p| p.to_string_lossy().into_owned())
}

fn status_char(status: Delta) -> char {
    match status {
        Delta::Added => 'A',
        Delta::Deleted => 'D',
        Delta::Modified => 'M',
        Delta::Renamed => 'R',
        Delta::Copied => 'C',
        Delta::Typechange => 'T',
        _ => '?',
    }
}

/// Formats a unix timestamp as `YYYY-MM-DD` (UTC) without pulling in a date crate.
fn format_time(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's days-from-civil inverse (proleptic Gregorian, UTC).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
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
