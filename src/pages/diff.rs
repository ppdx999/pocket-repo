use maud::{html, Markup};
use serde::{Deserialize, Serialize};

use crate::framework::{Page, PageContext, Update};
use crate::git::{self, DiffLineRow, DiffTarget, DiffView, FileDiff};
use crate::pages::breadcrumb;

const HISTORY_LIMIT: usize = 100;

pub struct DiffPage;

#[derive(Serialize, Deserialize)]
pub struct Model {
    pub repo: String,
    #[serde(default)]
    pub rev: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Event {}

crate::impl_event_display!(Event);

impl Page for DiffPage {
    type Model = Model;
    type Event = Event;

    fn path() -> &'static str {
        "/repo/{repo}/diff"
    }

    fn init(ctx: &PageContext) -> Model {
        let rev = ctx
            .query("rev")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        Model {
            repo: ctx.param_or_empty("repo").to_string(),
            rev,
        }
    }

    fn update(model: Model, _event: Event, _params: serde_json::Value) -> Update<Model> {
        Update::Render(model)
    }

    fn view(model: &Model) -> Markup {
        render(model)
    }
}

fn diff_url(repo: &str, rev: Option<&str>) -> String {
    match rev {
        None => format!("/repo/{repo}/diff"),
        Some(oid) => format!("/repo/{repo}/diff?rev={oid}"),
    }
}

fn render(model: &Model) -> Markup {
    let repo = &model.repo;
    let history = git::history(repo, HISTORY_LIMIT).unwrap_or_default();

    // Timeline: index 0 = working tree, index k = history[k-1].
    let timeline_len = history.len() + 1;
    let cur_index: Option<usize> = match &model.rev {
        None => Some(0),
        Some(oid) => history
            .iter()
            .position(|r| r.oid.starts_with(oid))
            .map(|i| i + 1),
    };
    let rev_at = |idx: usize| -> Option<String> {
        history.get(idx.wrapping_sub(1)).map(|r| r.oid.clone())
    };

    let (newer_url, older_url) = match cur_index {
        Some(i) => (
            (i > 0).then(|| diff_url(repo, rev_at(i - 1).as_deref())),
            (i + 1 < timeline_len).then(|| diff_url(repo, rev_at(i + 1).as_deref())),
        ),
        None => (None, None),
    };

    let (title, subtitle) = match cur_index {
        Some(0) => ("Working tree".to_string(), "Uncommitted changes".to_string()),
        Some(i) => {
            let r = &history[i - 1];
            (
                format!("{}  {}", r.short, r.summary),
                format!("{} · {}", r.author, r.when),
            )
        }
        None => (
            model.rev.clone().unwrap_or_default(),
            "commit outside recent history".to_string(),
        ),
    };

    let target = match &model.rev {
        None => DiffTarget::WorkingTree,
        Some(oid) => DiffTarget::Commit(oid.clone()),
    };
    let diff = git::compute_diff(repo, &target);

    html! {
        div id="maudliver-root" class="page" {
            header class="app-header" {
                a href="/" class="home-link" { "PocketRepo" }
                (breadcrumb(repo, "", None, false))
                div class="diff-nav" {
                    (nav_button("← Newer", newer_url.as_deref()))
                    div class="diff-pos" {
                        div class="diff-title" { (title) }
                        @if !subtitle.is_empty() { div class="diff-sub" { (subtitle) } }
                    }
                    (nav_button("Older →", older_url.as_deref()))
                }
            }
            main {
                @match diff {
                    Ok(view) => (render_diff(&view)),
                    Err(e) => p class="error" { (e.to_string()) },
                }
            }
        }
    }
}

fn nav_button(label: &str, url: Option<&str>) -> Markup {
    html! {
        @match url {
            Some(u) => a class="nav-btn" href=(u) { (label) },
            None => span class="nav-btn disabled" { (label) },
        }
    }
}

fn render_diff(view: &DiffView) -> Markup {
    if view.files.is_empty() {
        return html! { p class="notice" { "No changes." } };
    }
    html! {
        @for file in &view.files {
            details class="diff-file" {
                summary class="diff-file-header" {
                    span class=(format!("diff-status s-{}", file.status)) { (file.status) }
                    span class="diff-path" { (file_label(file)) }
                    span class="diff-stat" {
                        @if file.additions > 0 { span class="add" { "+" (file.additions) } }
                        @if file.deletions > 0 { span class="del" { "−" (file.deletions) } }
                    }
                }
                @if file.is_binary {
                    div class="diff-note" { "Binary file" }
                } @else if file.hunks.is_empty() {
                    div class="diff-note" { "No content changes" }
                } @else {
                    div class="diff-body" {
                        @for hunk in &file.hunks {
                            div class="hunk-header" { (hunk.header) }
                            @for line in &hunk.lines {
                                div class=(line_class(line.origin)) {
                                    span class="ln" { (lineno(line)) }
                                    span class="dl-sign" { (sign(line.origin)) }
                                    span class="dl-code" { (line.content) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn file_label(f: &FileDiff) -> String {
    match (&f.old_path, &f.new_path) {
        (Some(o), Some(n)) if o != n => format!("{o} → {n}"),
        (_, Some(n)) => n.clone(),
        (Some(o), None) => o.clone(),
        (None, None) => "?".to_string(),
    }
}

fn line_class(origin: char) -> &'static str {
    match origin {
        '+' => "dl add",
        '-' => "dl del",
        _ => "dl ctx",
    }
}

fn sign(origin: char) -> &'static str {
    match origin {
        '+' => "+",
        '-' => "−",
        _ => " ",
    }
}

fn lineno(line: &DiffLineRow) -> String {
    match line.new_lineno.or(line.old_lineno) {
        Some(n) => n.to_string(),
        None => String::new(),
    }
}
