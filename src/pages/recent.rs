use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};

/// The recent-files list is stored client-side (localStorage) and rendered by
/// app.js, so this page only ships an empty shell for the script to populate.
pub struct RecentPage;

#[derive(Serialize, Deserialize)]
pub struct Model {}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for RecentPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/recent"
    }

    fn init(_ctx: &PageContext) -> Model {
        Model {}
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(_model: &Model) -> Markup {
        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    a href="/" class="home-link" { "PocketRepo" }
                    h1 { "Recent files" }
                }
                main {
                    ul id="recent-list" class="search-results" {}
                    p id="recent-empty" class="notice" hidden { "No recently viewed files yet." }
                }
            }
        }
    }
}
