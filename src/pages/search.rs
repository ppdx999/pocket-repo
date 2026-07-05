use maud::{html, Markup};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git;
use crate::pages::{
    branch_chip, breadcrumb, copy_button, recent_link, ref_query, search_bar, split_path,
};

/// Cap on rendered results to keep the page light on large repos.
const MAX_RESULTS: usize = 200;

pub struct SearchPage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
    pub query: String,
    #[serde(default, rename = "ref")]
    pub ref_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for SearchPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/repo/{repo}/search"
    }

    fn init(ctx: &PageContext) -> Model {
        Model {
            repo: ctx.param_or_empty("repo").to_string(),
            query: ctx.query("q").unwrap_or("").to_string(),
            ref_name: ctx.query("ref").filter(|s| !s.is_empty()).map(String::from),
        }
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        let repo = &model.repo;
        let query = model.query.trim();
        let ref_name = model.ref_name.as_deref();
        let rq = ref_query(ref_name);
        let branch = model
            .ref_name
            .clone()
            .or_else(|| git::head_branch(repo))
            .unwrap_or_else(|| "HEAD".to_string());

        html! {
            div id="maudliver-root" class="page" {
                header class="app-header" {
                    div class="header-top" {
                        a href="/" class="home-link" { "PocketRepo" }
                        div class="header-actions" {
                            (recent_link())
                        }
                    }
                    div class="breadcrumb-row" {
                        (breadcrumb(repo, "", ref_name, false))
                        (branch_chip(repo, &branch))
                    }
                    (search_bar(repo, &model.query, ref_name))
                }
                main {
                    @if query.is_empty() {
                        p class="notice" { "Type a filename to search " (repo) "." }
                    } @else {
                        @match git::list_files(repo, ref_name) {
                            Ok(all) => (results(repo, query, &rq, &all)),
                            Err(e) => p class="error" { (e.to_string()) },
                        }
                    }
                }
            }
        }
    }
}

fn results(repo: &str, query: &str, rq: &str, all: &[String]) -> Markup {
    // Fuzzy match + rank via nucleo (the matcher Helix uses). `match_paths`
    // tunes scoring for file paths; `match_list` filters non-matches and
    // returns hits sorted by descending score.
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let matches = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
        .match_list(all.iter().map(String::as_str), &mut matcher);
    let total = matches.len();
    let shown = &matches[..total.min(MAX_RESULTS)];

    html! {
        p class="search-summary" {
            @if total == 0 {
                "No files match “" (query) "”."
            } @else if total > MAX_RESULTS {
                "Showing " (MAX_RESULTS) " of " (total) " matches for “" (query) "”."
            } @else {
                (total) " match" @if total != 1 { "es" } " for “" (query) "”."
            }
        }
        ul class="search-results" {
            @for (path, _score) in shown {
                @let (dir, base) = split_path(path);
                li class="result" {
                    a href=(format!("/repo/{repo}/blob/{path}{rq}")) {
                        @if !dir.is_empty() { span class="path-dir" { (dir) } }
                        span class="path-base" { (base) }
                    }
                    (copy_button(path))
                }
            }
        }
    }
}
