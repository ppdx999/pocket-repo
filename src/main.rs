mod config;
mod framework;
mod git;
mod highlight;
mod pages;

use std::path::PathBuf;

use axum::http::{header, StatusCode};
use axum::Router;

use framework::handler::page_routes;
use pages::blob::BlobPage;
use pages::diff::DiffPage;
use pages::recent::RecentPage;
use pages::repos::ReposPage;
use pages::search::SearchPage;
use pages::tree::TreePage;

async fn runtime_js() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../static/runtime.js"),
    )
}

async fn app_css() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("../static/app.css"),
    )
}

async fn app_js() -> ([(header::HeaderName, &'static str); 1], &'static str) {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../static/app.js"),
    )
}

#[tokio::main]
async fn main() {
    // Repositories to serve are the CLI arguments; default to the current dir.
    let mut paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }
    config::init(paths);

    let repos = config::repos();
    if repos.is_empty() {
        eprintln!("warning: no git repositories to serve");
    } else {
        println!("Serving {} repositor{}:", repos.len(), if repos.len() == 1 { "y" } else { "ies" });
        for repo in &repos {
            println!("  {}  ({})", repo.name, repo.path.display());
        }
    }

    let app = Router::new()
        .merge(page_routes::<ReposPage>())
        .merge(page_routes::<TreePage>())
        .merge(page_routes::<BlobPage>())
        .merge(page_routes::<SearchPage>())
        .merge(page_routes::<DiffPage>())
        .merge(page_routes::<RecentPage>())
        .route("/static/runtime.js", axum::routing::get(runtime_js))
        .route("/static/app.css", axum::routing::get(app_css))
        .route("/static/app.js", axum::routing::get(app_js))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found") });

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
