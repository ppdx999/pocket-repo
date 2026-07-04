//! Configuration: which repositories to serve and how to bind.
//!
//! Sources, merged in this order: a TOML config file (`--config` or the default
//! `~/.config/pocket-repo/config.toml`), then positional CLI paths. Repos can be
//! listed explicitly or discovered by scanning `scan_roots` (handy for ghq-style
//! layouts). The resulting registry is read-only for the process lifetime, so a
//! global `OnceLock` lets `view()` reach it without threading state through
//! maudliver's pure page functions.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

static REGISTRY: OnceLock<BTreeMap<String, PathBuf>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
}

/// Server bind settings resolved from config + CLI.
pub struct Settings {
    pub bind: String,
    pub port: u16,
}

/// Options gathered from the command line.
#[derive(Default)]
pub struct CliOptions {
    pub config_path: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub bind: Option<String>,
    pub port: Option<u16>,
}

#[derive(Deserialize, Default)]
struct FileConfig {
    bind: Option<String>,
    port: Option<u16>,
    #[serde(default)]
    repos: Vec<RepoEntry>,
    #[serde(default)]
    scan_roots: Vec<String>,
}

#[derive(Deserialize)]
struct RepoEntry {
    path: String,
    #[serde(default)]
    name: Option<String>,
}

/// Loads config + CLI, builds the repository registry, and returns bind settings.
pub fn load(cli: CliOptions) -> Settings {
    let file = read_config_file(cli.config_path.as_deref());

    // (explicit name, path) candidates, in priority order.
    let mut candidates: Vec<(Option<String>, PathBuf)> = Vec::new();
    for entry in &file.repos {
        candidates.push((entry.name.clone(), expand_tilde(&entry.path)));
    }
    for root in &file.scan_roots {
        let mut found = Vec::new();
        scan_git_repos(&expand_tilde(root), &mut found, 8);
        found.sort();
        candidates.extend(found.into_iter().map(|p| (None, p)));
    }
    candidates.extend(cli.paths.iter().cloned().map(|p| (None, p)));

    if candidates.is_empty() {
        candidates.push((None, PathBuf::from(".")));
    }

    build_registry(candidates);

    Settings {
        bind: cli.bind.or(file.bind).unwrap_or_else(|| "0.0.0.0".to_string()),
        port: cli.port.or(file.port).unwrap_or(3000),
    }
}

fn read_config_file(explicit: Option<&Path>) -> FileConfig {
    let path = match explicit.map(PathBuf::from).or_else(default_config_path) {
        Some(p) if p.exists() => p,
        _ => return FileConfig::default(),
    };
    match fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("config: failed to parse {}: {e}", path.display());
                FileConfig::default()
            }
        },
        Err(e) => {
            eprintln!("config: failed to read {}: {e}", path.display());
            FileConfig::default()
        }
    }
}

fn build_registry(candidates: Vec<(Option<String>, PathBuf)>) {
    let mut map: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for (name_opt, path) in candidates {
        let canonical = path.canonicalize().unwrap_or(path.clone());
        if !canonical.is_dir() {
            eprintln!("skipping {}: not a directory", canonical.display());
            continue;
        }
        if git2::Repository::open(&canonical).is_err() {
            eprintln!("skipping {}: not a git repository", canonical.display());
            continue;
        }
        if !seen.insert(canonical.clone()) {
            continue; // same repo listed twice
        }
        let base = name_opt.unwrap_or_else(|| basename(&canonical));
        let name = unique_name(&map, base, &canonical);
        map.insert(name, canonical);
    }

    let _ = REGISTRY.set(map);
}

/// Ensures a URL-safe, unique repo name. On collision, prefixes the parent
/// directory name (e.g. ghq's `owner`), then falls back to a numeric suffix.
fn unique_name(map: &BTreeMap<String, PathBuf>, base: String, path: &Path) -> String {
    if !map.contains_key(&base) {
        return base;
    }
    if let Some(parent) = path.parent().and_then(Path::file_name).and_then(|s| s.to_str()) {
        let alt = format!("{parent}-{base}");
        if !map.contains_key(&alt) {
            return alt;
        }
    }
    let mut i = 2;
    loop {
        let alt = format!("{base}-{i}");
        if !map.contains_key(&alt) {
            return alt;
        }
        i += 1;
    }
}

/// Recursively finds git repositories under `root`, not descending into a repo
/// once found. Skips hidden directories; bounded by `depth`.
fn scan_git_repos(root: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth == 0 || !root.is_dir() {
        return;
    }
    if root.join(".git").exists() {
        out.push(root.to_path_buf());
        return;
    }
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        scan_git_repos(&path, out, depth - 1);
    }
}

fn default_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".config")))?;
    Some(base.join("pocket-repo").join("config.toml"))
}

fn expand_tilde(s: &str) -> PathBuf {
    if s == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(s));
    }
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn basename(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".to_string())
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
