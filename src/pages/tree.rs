use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git::{self, Resolved};
use crate::pages::{breadcrumb, copy_button, join_path, search_bar};

pub struct TreePage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub enum Event {}

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
        }
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        let repo = &model.repo;
        let path = &model.path;

        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    a href="/" class="home-link" { "PocketRepo" }
                    (breadcrumb(repo, path, false))
                    (search_bar(repo, ""))
                }
                main {
                    @match git::resolve(repo, path) {
                        Ok(Resolved::Dir(entries)) => {
                            ul class="tree" {
                                @if !path.is_empty() {
                                    li class="entry dir" {
                                        a class="entry-link" href=(parent_url(repo, path)) {
                                            span class="icon" { ".." }
                                        }
                                    }
                                }
                                @for entry in &entries {
                                    @let child = join_path(path, &entry.name);
                                    @let url = if entry.is_dir {
                                        format!("/repo/{repo}/tree/{child}")
                                    } else {
                                        format!("/repo/{repo}/blob/{child}")
                                    };
                                    li class=(if entry.is_dir { "entry dir" } else { "entry file" }) {
                                        a class="entry-link" href=(url) {
                                            span class="icon" { (if entry.is_dir { "📁" } else { "📄" }) }
                                            span class="name" { (entry.name) }
                                        }
                                        (copy_button(&child))
                                    }
                                }
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

/// URL of the parent directory of `path` within `repo`.
fn parent_url(repo: &str, path: &str) -> String {
    match path.rsplit_once('/') {
        Some((parent, _)) => format!("/repo/{repo}/tree/{parent}"),
        None => format!("/repo/{repo}/tree"),
    }
}
