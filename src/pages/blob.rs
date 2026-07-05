use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git::{self, Resolved};
use crate::highlight;
use crate::pages::{breadcrumb, branch_chip, copy_button, recent_link, ref_query, search_link};

pub struct BlobPage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
    pub path: String,
    #[serde(default, rename = "ref")]
    pub ref_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for BlobPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/repo/{repo}/blob/{*path}"
    }

    fn init(ctx: &PageContext) -> Model {
        Model {
            repo: ctx.param_or_empty("repo").to_string(),
            path: ctx.param_or_empty("path").trim_matches('/').to_string(),
            ref_name: ctx.query("ref").filter(|s| !s.is_empty()).map(String::from),
        }
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        let repo = &model.repo;
        let path = &model.path;
        let ref_name = model.ref_name.as_deref();
        let rq = ref_query(ref_name);
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let branch = model
            .ref_name
            .clone()
            .or_else(|| git::head_branch(repo))
            .unwrap_or_else(|| "HEAD".to_string());

        html! {
            div id="maudliver-root" class="page" data-recent-repo=(repo) data-recent-path=(path) {
                header class="app-header" {
                    div class="header-top" {
                        a href="/" class="home-link" { "PocketRepo" }
                        div class="header-actions" {
                            (search_link(repo, ref_name))
                            (recent_link())
                            (copy_button(path))
                        }
                    }
                    div class="breadcrumb-row" {
                        (breadcrumb(repo, path, ref_name, true))
                        (branch_chip(repo, &branch))
                    }
                }
                main {
                    @match git::resolve(repo, ref_name, path) {
                        Ok(Resolved::File(blob)) => {
                            @if blob.is_binary {
                                p class="notice" { "Binary file not shown (" (blob.content.len()) " bytes)" }
                            } @else {
                                @match String::from_utf8(blob.content) {
                                    Ok(text) => {
                                        div id="file-content" class="file-content" {
                                            (highlight::to_html(&text, file_name))
                                        }
                                    }
                                    Err(_) => {
                                        p class="notice" { "File is not valid UTF-8 and cannot be displayed." }
                                    }
                                }
                            }
                        }
                        Ok(Resolved::Dir(_)) => {
                            p class="notice" {
                                "This path is a directory. "
                                a href=(format!("/repo/{repo}/tree/{path}{rq}")) { "Browse directory" }
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
