pub mod blob;
pub mod repos;
pub mod tree;

use maud::{html, Markup};

/// Joins a directory path and a child name into a clean repo-relative path.
pub fn join_path(base: &str, name: &str) -> String {
    let base = base.trim_matches('/');
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{base}/{name}")
    }
}

/// A single breadcrumb segment: its label and, unless it's the current leaf,
/// the tree URL it links to.
struct Crumb {
    label: String,
    href: Option<String>,
}

/// Breadcrumb navigation: `repo / dir / subdir`, each segment a link.
///
/// `leaf_is_file` controls whether the final segment is a plain label (a file,
/// which has no tree URL) or a link.
pub fn breadcrumb(repo: &str, path: &str, leaf_is_file: bool) -> Markup {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let mut crumbs = Vec::new();
    let mut acc = String::new();
    for (i, seg) in segments.iter().enumerate() {
        acc = join_path(&acc, seg);
        let is_last = i + 1 == segments.len();
        let href = if is_last && leaf_is_file {
            None
        } else {
            Some(format!("/repo/{repo}/tree/{acc}"))
        };
        crumbs.push(Crumb {
            label: seg.to_string(),
            href,
        });
    }

    html! {
        nav id="breadcrumb" class="breadcrumb" {
            a href=(format!("/repo/{repo}/tree")) { (repo) }
            @for crumb in &crumbs {
                span class="sep" { "/" }
                @match &crumb.href {
                    Some(href) => a href=(href) { (crumb.label) },
                    None => span class="crumb-current" { (crumb.label) },
                }
            }
        }
    }
}
