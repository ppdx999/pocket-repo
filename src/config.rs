//! Configuration: which repositories to serve and how to bind.
//!
//! Sources, merged in this order: a TOML config file (`--config` or the default
//! `~/.config/pocket-repo/config.toml`), then positional CLI paths. Repos can be
//! listed explicitly or discovered by scanning `scan_roots` (handy for ghq-style
//! layouts).
//!
//! The registry is rebuilt on demand (not frozen at startup) so repositories
//! added under `scan_roots` while the server runs show up without a restart:
//! [`repos`] rescans on every home-page render, and [`repo_path`] rescans on a
//! cache miss. Sources are captured once; a `RwLock` holds the current registry.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use serde::Deserialize;

/// The inputs needed to (re)build the registry — captured once at startup.
struct Sources {
    repos: Vec<RepoEntry>,
    scan_roots: Vec<String>,
    cli_paths: Vec<PathBuf>,
}

static SOURCES: OnceLock<Sources> = OnceLock::new();
static REGISTRY: OnceLock<RwLock<BTreeMap<String, PathBuf>>> = OnceLock::new();

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

/// Loads config + CLI, captures the sources, builds the initial registry, and
/// returns bind settings.
pub fn load(cli: CliOptions) -> Settings {
    let file = read_config_file(cli.config_path.as_deref());

    let settings = Settings {
        bind: cli.bind.or(file.bind).unwrap_or_else(|| "0.0.0.0".to_string()),
        port: cli.port.or(file.port).unwrap_or(3000),
    };

    let _ = SOURCES.set(Sources {
        repos: file.repos,
        scan_roots: file.scan_roots,
        cli_paths: cli.paths,
    });
    rebuild();

    settings
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

/// Rescans the sources and replaces the current registry.
fn rebuild() {
    let sources = match SOURCES.get() {
        Some(s) => s,
        None => return,
    };

    // (explicit name, path) candidates, in priority order.
    let mut candidates: Vec<(Option<String>, PathBuf)> = Vec::new();
    for entry in &sources.repos {
        candidates.push((entry.name.clone(), expand_tilde(&entry.path)));
    }
    for root in &sources.scan_roots {
        let mut found = Vec::new();
        scan_git_repos(&expand_tilde(root), &mut found, 8);
        found.sort();
        candidates.extend(found.into_iter().map(|p| (None, p)));
    }
    candidates.extend(sources.cli_paths.iter().cloned().map(|p| (None, p)));

    if candidates.is_empty() {
        candidates.push((None, PathBuf::from(".")));
    }

    let map = build_map(candidates);
    *registry().write().unwrap() = map;
}

fn build_map(candidates: Vec<(Option<String>, PathBuf)>) -> BTreeMap<String, PathBuf> {
    let mut map: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for (name_opt, path) in candidates {
        let canonical = path.canonicalize().unwrap_or(path.clone());
        if !canonical.is_dir() {
            continue;
        }
        if git2::Repository::open(&canonical).is_err() {
            continue;
        }
        if !seen.insert(canonical.clone()) {
            continue; // same repo listed twice
        }
        let base = name_opt.unwrap_or_else(|| basename(&canonical));
        let name = unique_name(&map, base, &canonical);
        map.insert(name, canonical);
    }

    map
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

fn registry() -> &'static RwLock<BTreeMap<String, PathBuf>> {
    REGISTRY.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// The current repositories. Rescans first so repos added at runtime appear.
pub fn repos() -> Vec<RepoInfo> {
    rebuild();
    registry()
        .read()
        .unwrap()
        .iter()
        .map(|(name, path)| RepoInfo {
            name: name.clone(),
            path: path.clone(),
        })
        .collect()
}

/// The on-disk path for a repo name. Rescans once on a miss, so a
/// recently-added repo resolves even if the home page wasn't reloaded.
pub fn repo_path(name: &str) -> Option<PathBuf> {
    if let Some(path) = registry().read().unwrap().get(name).cloned() {
        return Some(path);
    }
    rebuild();
    registry().read().unwrap().get(name).cloned()
}
