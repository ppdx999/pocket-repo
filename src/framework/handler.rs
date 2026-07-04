use axum::extract::{RawPathParams, RawQuery};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::get;
use axum::Json;
use axum::Router;
use maud::html;
use serde_json::Value;

use super::context::PageContext;
use super::request::PageRequest;
use super::response::{PageResponse, PatchEntry};
use super::Page;

/// Registers GET (initial HTML) and POST (event handling) handlers for every
/// route pattern the page declares.
pub fn page_routes<P: Page + 'static>() -> Router {
    let mut router = Router::new();
    for pattern in P::route_patterns() {
        router = router.route(pattern, get(render_page::<P>).post(handle_event::<P>));
    }
    router
}

async fn render_page<P: Page>(params: RawPathParams, RawQuery(query): RawQuery) -> Html<String> {
    let ctx = PageContext::new(&params, query.as_deref());
    let model = P::init(&ctx);
    let content = P::view(&model);
    let model_json = serde_json::to_string(&model).unwrap();

    let page = html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                meta name="color-scheme" content="light dark";
                title { "PocketRepo" }
                link rel="stylesheet" href="/static/app.css";
            }
            body {
                (content)
                script data-model=(model_json) src="/static/runtime.js" {}
            }
        }
    };

    Html(page.into_string())
}

async fn handle_event<P: Page>(
    Json(req): Json<PageRequest>,
) -> Result<Json<PageResponse>, StatusCode> {
    let model: P::Model = if req.model.is_null() {
        // A reload-less event before any model exists shouldn't normally happen,
        // but fall back to a context-free init so we degrade gracefully.
        P::init(&PageContext::default())
    } else {
        serde_json::from_value(req.model).map_err(|_| StatusCode::BAD_REQUEST)?
    };

    let event: P::Event =
        serde_json::from_value(Value::String(req.event)).map_err(|_| StatusCode::BAD_REQUEST)?;

    let old_markup = P::view(&model);

    match P::update(model, event, req.params) {
        super::Update::Render(new_model) => {
            let new_markup = P::view(&new_model);
            let model_value =
                serde_json::to_value(&new_model).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let patches = super::diff::diff(&old_markup, &new_markup)
                .into_iter()
                .map(|p| PatchEntry {
                    id: p.id,
                    html: p.html,
                })
                .collect();

            Ok(Json(PageResponse {
                patches,
                model: model_value,
                redirect: None,
            }))
        }
        super::Update::Redirect(url) => Ok(Json(PageResponse {
            patches: vec![],
            model: serde_json::Value::Null,
            redirect: Some(url),
        })),
    }
}
