//! Server-side syntax highlighting via syntect.
//!
//! Highlighting happens on the server so the client never ships a highlighter —
//! a good fit for maudliver's server-rendered model. The `SyntaxSet`/`ThemeSet`
//! are loaded once and shared.

use std::sync::OnceLock;

use maud::{Markup, PreEscaped};
use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

fn syntaxes() -> &'static SyntaxSet {
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let mut ts = ThemeSet::load_defaults();
        ts.themes
            .remove("base16-ocean.dark")
            .expect("bundled theme present")
    })
}

/// Highlights `code` as HTML, picking a syntax from the file name's extension.
/// Falls back to plain (escaped) text when no syntax matches.
pub fn to_html(code: &str, file_name: &str) -> Markup {
    let ss = syntaxes();
    let syntax = file_name
        .rsplit('.')
        .next()
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    match highlighted_html_for_string(code, ss, syntax, theme()) {
        Ok(html) => PreEscaped(html),
        Err(_) => PreEscaped(maud::html! { pre { code { (code) } } }.into_string()),
    }
}
