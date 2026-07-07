mod config;
mod framework;
mod git;
mod highlight;
mod pages;

use std::path::PathBuf;

use axum::http::{header, StatusCode};
use axum::Router;
use tower_http::compression::CompressionLayer;

use framework::handler::page_routes;
use pages::blob::BlobPage;
use pages::diff::DiffPage;
use pages::recent::RecentPage;
use pages::refs::RefsPage;
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

/// Serves a blob's raw bytes with a guessed content type — used by the file
/// view to render images (and available for direct download of any file).
async fn raw_blob(
    axum::extract::Path((repo, path)): axum::extract::Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let ref_name = params.get("ref").map(String::as_str).filter(|s| !s.is_empty());
    match git::resolve(&repo, ref_name, &path) {
        Ok(git::Resolved::File(blob)) => (
            [
                (header::CONTENT_TYPE, pages::content_type_for(&path)),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            blob.content,
        )
            .into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

const HELP: &str = "\
pocket-repo — browse Git repositories from your phone

USAGE:
    pocket-repo [OPTIONS] [REPO_PATH]...

OPTIONS:
    -c, --config <PATH>   Config file (default: ~/.config/pocket-repo/config.toml)
        --bind <ADDR>     Bind address (default: 0.0.0.0)
        --port <PORT>     Port (default: 3000)
    -h, --help            Print this help
    -V, --version         Print version

REPO_PATH arguments are served in addition to any repos from the config file.
Config keys: bind, port, repos = [{ path, name }], scan_roots = [\"~/ghq\"].";

/// Minimal flag parsing: `-c/--config <path>`, `--bind <addr>`, `--port <n>`,
/// `-h/--help`, `-V/--version`, and positional repository paths.
fn parse_args() -> config::CliOptions {
    let mut opts = config::CliOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("pocket-repo {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-c" | "--config" => opts.config_path = args.next().map(PathBuf::from),
            "--bind" => opts.bind = args.next(),
            "--port" => opts.port = args.next().and_then(|v| v.parse().ok()),
            _ => opts.paths.push(PathBuf::from(arg)),
        }
    }
    opts
}

#[tokio::main]
async fn main() {
    let settings = config::load(parse_args());

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
        .merge(page_routes::<RefsPage>())
        .route("/static/runtime.js", axum::routing::get(runtime_js))
        .route("/static/app.css", axum::routing::get(app_css))
        .route("/static/app.js", axum::routing::get(app_js))
        .route("/repo/{repo}/raw/{*path}", axum::routing::get(raw_blob))
        .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found") })
        // Negotiates br / zstd / gzip from Accept-Encoding — worthwhile since
        // pages (highlighted files, diffs) are repetitive text served over
        // Tailscale, often via a mobile link.
        .layer(CompressionLayer::new());

    let addr = format!("{}:{}", settings.bind, settings.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
