use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::framework::{Page, PageContext, Update};
use crate::pages::recent_link;

pub struct ReposPage;

#[derive(Serialize, Deserialize)]
pub struct Model {}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for ReposPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/"
    }

    fn init(_ctx: &PageContext) -> Model {
        Model {}
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(_model: &Model) -> Markup {
        let repos = config::repos();
        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    div class="header-top" {
                        h1 { "PocketRepo" }
                        div class="header-actions" {
                            (recent_link())
                        }
                    }
                }
                main {
                    @if repos.is_empty() {
                        p class="empty" { "No repositories configured. Start the server with paths: " code { "pocket-repo <path>..." } }
                    } @else {
                        div class="repo-filter-bar" {
                            input type="search" id="repo-filter" placeholder="Filter repositories…"
                                autocomplete="off" autocapitalize="off" spellcheck="false";
                        }
                        ul id="repo-list" class="repo-list" {
                            @for repo in &repos {
                                li data-name=(repo.name) {
                                    a href=(format!("/repo/{}/tree", repo.name)) class="repo-link" {
                                        span class="repo-name" { (repo.name) }
                                        span class="repo-path" { (repo.path.display().to_string()) }
                                    }
                                }
                            }
                        }
                        p id="repo-filter-empty" class="notice" hidden { "No matching repositories." }
                    }
                }
            }
        }
    }
}
