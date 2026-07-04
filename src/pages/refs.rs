use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git;
use crate::pages::breadcrumb;

pub struct RefsPage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for RefsPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/repo/{repo}/refs"
    }

    fn init(ctx: &PageContext) -> Model {
        Model {
            repo: ctx.param_or_empty("repo").to_string(),
        }
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        let repo = &model.repo;
        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    a href="/" class="home-link" { "PocketRepo" }
                    (breadcrumb(repo, "", None, false))
                    h1 { "Branches & tags" }
                }
                main {
                    @match git::list_refs(repo) {
                        Ok(refs) => {
                            ul class="ref-list" {
                                @for r in &refs {
                                    li {
                                        a class="ref-link" href=(format!("/repo/{repo}/tree?ref={}", r.name)) {
                                            span class="ref-name" { (r.name) }
                                            @if r.is_head {
                                                span class="ref-badge head" { "current" }
                                            } @else if r.is_tag {
                                                span class="ref-badge tag" { "tag" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => p class="error" { (e.to_string()) },
                    }
                }
            }
        }
    }
}
