pub mod blob;
pub mod repos;
pub mod tree;

use maud::{html, Markup};

/// A button that copies `path` (a repo-root-relative path) to the clipboard.
/// The click is handled client-side by `static/app.js` via the `data-copy`
/// attribute; the double-square glyph is an inline SVG so it inherits color.
pub fn copy_button(path: &str) -> Markup {
    html! {
        button type="button" class="copy-btn"
            data-copy=(path)
            title="Copy path"
            aria-label=(format!("Copy path: {path}")) {
            svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                rect x="8" y="8" width="14" height="14" rx="2" ry="2" {}
                path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" {}
            }
        }
    }
}

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
