//! maudliver — a stateless, server-driven UI framework (Elm Architecture over HTTP).
//!
//! Vendored from <https://github.com/ppdx999/maudliver> and adapted for PocketRepo.
//! Changes vs upstream:
//!   * `init` receives a [`PageContext`] so pages can read URL/query state.
//!   * `Page::route_patterns` lets one page register several concrete routes
//!     (e.g. `/repo/{repo}/tree` and `/repo/{repo}/tree/{*path}`).

pub mod context;
pub mod diff;
pub mod handler;
pub mod request;
pub mod response;

pub use context::PageContext;

use maud::Markup;
use serde::{de::DeserializeOwned, Serialize};

pub enum Update<M> {
    Render(M),
    Redirect(String),
}

pub trait Page {
    type Model: Serialize + DeserializeOwned;
    type Event: Serialize + DeserializeOwned + std::fmt::Display;

    /// Primary route pattern (axum syntax). Also used as the page's identity.
    fn path() -> &'static str;

    /// All concrete GET/POST route patterns this page answers on.
    /// Defaults to just [`Page::path`]; override to add optional-wildcard variants.
    fn route_patterns() -> Vec<&'static str> {
        vec![Self::path()]
    }

    fn init(ctx: &PageContext) -> Self::Model;
    fn update(model: Self::Model, event: Self::Event, params: serde_json::Value) -> Update<Self::Model>;
    fn view(model: &Self::Model) -> Markup;
}

/// Implements `Display` for an Event enum using its serde serialization.
/// This allows using event variants directly in Maud templates:
/// ```ignore
/// button data-event=(Event::Increment) { "+1" }
/// ```
#[macro_export]
macro_rules! impl_event_display {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let v = serde_json::to_value(self).unwrap();
                f.write_str(v.as_str().unwrap())
            }
        }
    };
}
