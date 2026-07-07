pub mod blob;
pub mod diff;
pub mod recent;
pub mod refs;
pub mod repos;
pub mod search;
pub mod tree;

use maud::{html, Markup};

/// A `?ref=<name>` query suffix to carry the selected branch/tag through links,
/// or `""` when viewing the default `HEAD`.
pub fn ref_query(ref_name: Option<&str>) -> String {
    match ref_name {
        Some(r) if !r.is_empty() => format!("?ref={r}"),
        _ => String::new(),
    }
}

/// A chip showing the current branch/ref, linking to the ref picker.
pub fn branch_chip(repo: &str, name: &str) -> Markup {
    html! {
        a class="branch-chip" href=(format!("/repo/{repo}/refs"))
            title="Switch branch or tag" {
            svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                line x1="6" y1="3" x2="6" y2="15" {}
                circle cx="18" cy="6" r="3" {}
                circle cx="6" cy="18" r="3" {}
                path d="M18 9a9 9 0 0 1-9 9" {}
            }
            span class="branch-name" { (name) }
        }
    }
}

/// An always-available file-search box that GETs to the repo's search page.
/// `query` pre-fills the input; `ref_name` is preserved as a hidden field.
pub fn search_bar(repo: &str, query: &str, ref_name: Option<&str>) -> Markup {
    html! {
        form class="search-bar" method="get" action=(format!("/repo/{repo}/search")) {
            input type="search" name="q" value=(query) placeholder="Search files…"
                autocomplete="off" autocapitalize="off" spellcheck="false";
            @if let Some(r) = ref_name {
                input type="hidden" name="ref" value=(r);
            }
            button type="submit" class="search-btn" aria-label="Search" {
                svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                    fill="none" stroke="currentColor" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                    circle cx="11" cy="11" r="8" {}
                    path d="m21 21-4.3-4.3" {}
                }
            }
        }
    }
}

/// A compact magnifier icon linking to the repo's search page. Used in the file
/// view header where a full-width search bar would crowd the reading area.
pub fn search_link(repo: &str, ref_name: Option<&str>) -> Markup {
    html! {
        a class="icon-action" href=(format!("/repo/{repo}/search{}", ref_query(ref_name)))
            title="Search files" aria-label="Search files" {
            svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                circle cx="11" cy="11" r="8" {}
                path d="m21 21-4.3-4.3" {}
            }
        }
    }
}

/// A compact history icon linking to the global recent-files page. The recent
/// list itself lives in the browser's localStorage and is rendered by app.js.
pub fn recent_link() -> Markup {
    html! {
        a class="icon-action" href="/recent" title="Recent files" aria-label="Recent files" {
            svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" stroke-width="2"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" {
                path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" {}
                path d="M3 3v5h5" {}
                path d="M12 7v5l4 2" {}
            }
        }
    }
}

/// Guesses a content type from a file's extension (for raw byte serving).
pub fn content_type_for(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Whether a path should be rendered as an image in the file view.
pub fn is_image_path(path: &str) -> bool {
    content_type_for(path).starts_with("image/")
}

/// Splits a path into its directory prefix (with trailing slash, or "") and its
/// basename, so results can render the dir muted and the filename emphasized.
pub fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(i) => (&path[..=i], &path[i + 1..]),
        None => ("", path),
    }
}

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
pub fn breadcrumb(repo: &str, path: &str, ref_name: Option<&str>, leaf_is_file: bool) -> Markup {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let rq = ref_query(ref_name);

    let mut crumbs = Vec::new();
    let mut acc = String::new();
    for (i, seg) in segments.iter().enumerate() {
        acc = join_path(&acc, seg);
        let is_last = i + 1 == segments.len();
        let href = if is_last && leaf_is_file {
            None
        } else {
            Some(format!("/repo/{repo}/tree/{acc}{rq}"))
        };
        crumbs.push(Crumb {
            label: seg.to_string(),
            href,
        });
    }

    html! {
        nav id="breadcrumb" class="breadcrumb" {
            a href=(format!("/repo/{repo}/tree{rq}")) { (repo) }
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
