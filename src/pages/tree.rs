use std::collections::HashSet;

use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git::{self, Resolved};
use crate::pages::{breadcrumb, copy_button, join_path, recent_link, search_link};

pub struct TreePage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
    pub path: String,
    /// Repo-relative paths of directories expanded in-page. UI state only;
    /// `view()` re-reads git for each expanded dir.
    #[serde(default)]
    pub expanded: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    /// Toggle one directory's expansion (params: `path`).
    Toggle,
    /// Replace the whole expanded set (params: `paths`) — used by app.js to
    /// restore saved state from localStorage on load.
    RestoreExpanded,
}

crate::impl_event_display!(Event);

impl Page for TreePage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/repo/{repo}/tree"
    }

    fn route_patterns() -> Vec<&'static str> {
        vec!["/repo/{repo}/tree", "/repo/{repo}/tree/{*path}"]
    }

    fn init(ctx: &PageContext) -> Model {
        Model {
            repo: ctx.param_or_empty("repo").to_string(),
            path: ctx.param_or_empty("path").trim_matches('/').to_string(),
            expanded: Vec::new(),
        }
    }

    fn update(mut model: Model, event: Event, params: serde_json::Value) -> Update<Model> {
        match event {
            Event::Toggle => {
                if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                    match model.expanded.iter().position(|p| p == path) {
                        Some(i) => {
                            model.expanded.remove(i);
                        }
                        None => model.expanded.push(path.to_string()),
                    }
                }
            }
            Event::RestoreExpanded => {
                if let Some(arr) = params.get("paths").and_then(|v| v.as_array()) {
                    model.expanded = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                }
            }
        }
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        let repo = &model.repo;
        let path = &model.path;
        let expanded: HashSet<&str> = model.expanded.iter().map(String::as_str).collect();

        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    div class="header-top" {
                        a href="/" class="home-link" { "PocketRepo" }
                        div class="header-actions" {
                            (search_link(repo))
                            (recent_link())
                            a class="text-action" href=(format!("/repo/{repo}/diff")) { "Changes" }
                        }
                    }
                    (breadcrumb(repo, path, false))
                }
                main {
                    @match git::resolve(repo, path) {
                        Ok(Resolved::Dir(_)) => {
                            ul class="tree" {
                                @if !path.is_empty() {
                                    li class="entry dir" {
                                        div class="entry-row" {
                                            span class="entry-icon" { "↩" }
                                            a class="entry-link" href=(parent_url(repo, path)) {
                                                span class="name" { ".." }
                                            }
                                        }
                                    }
                                }
                                (entries_markup(repo, path, &expanded))
                            }
                        }
                        Ok(Resolved::File(_)) => {
                            p class="notice" {
                                "This path is a file. "
                                a href=(format!("/repo/{repo}/blob/{path}")) { "View file" }
                            }
                        }
                        Err(e) => {
                            p class="error" { (e.to_string()) }
                        }
                    }
                }
            }
        }
    }
}

/// Renders the `<li>` entries for `dir`, recursing into expanded subdirectories.
fn entries_markup(repo: &str, dir: &str, expanded: &HashSet<&str>) -> Markup {
    match git::resolve(repo, dir) {
        Ok(Resolved::Dir(entries)) => html! {
            @for entry in &entries {
                @let child = join_path(dir, &entry.name);
                @if entry.is_dir {
                    @let open = expanded.contains(child.as_str());
                    li id=(dir_id(&child)) class=(if open { "entry dir open" } else { "entry dir" }) {
                        div class="entry-row" {
                            button type="button" class="entry-icon toggle"
                                data-event=(Event::Toggle) data-param-path=(child)
                                aria-label="Expand/collapse" {
                                (if open { "📂" } else { "📁" })
                            }
                            a class="entry-link" href=(format!("/repo/{repo}/tree/{child}")) {
                                span class="name" { (entry.name) }
                            }
                            (copy_button(&child))
                        }
                        @if open {
                            div class="children" {
                                ul class="tree" { (entries_markup(repo, &child, expanded)) }
                            }
                        }
                    }
                } @else {
                    li class="entry file" {
                        div class="entry-row" {
                            span class="entry-icon" { "📄" }
                            a class="entry-link" href=(format!("/repo/{repo}/blob/{child}")) {
                                span class="name" { (entry.name) }
                            }
                            (copy_button(&child))
                        }
                    }
                }
            }
        },
        Ok(Resolved::File(_)) => html! {},
        Err(e) => html! { li class="entry" { span class="error" { (e.to_string()) } } },
    }
}

/// A CSS-selector-safe, unique, deterministic element id for a directory path
/// (maudliver's diff selects patches by `#id`, so slashes/dots aren't allowed).
fn dir_id(path: &str) -> String {
    let mut id = String::from("dir-");
    for byte in path.bytes() {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

/// URL of the parent directory of `path` within `repo`.
fn parent_url(repo: &str, path: &str) -> String {
    match path.rsplit_once('/') {
        Some((parent, _)) => format!("/repo/{repo}/tree/{parent}"),
        None => format!("/repo/{repo}/tree"),
    }
}
