use std::collections::HashMap;

use axum::extract::RawPathParams;

/// Request context handed to [`Page::init`](super::Page::init).
///
/// maudliver's original `init()` takes no arguments, which means a page cannot
/// know *which* URL it was loaded for. PocketRepo needs the repository name and
/// file path to live in the URL (so pages are bookmarkable and survive reloads),
/// so we extend `init` to receive this context built from the request's path
/// parameters and query string.
#[derive(Debug, Default, Clone)]
pub struct PageContext {
    params: HashMap<String, String>,
    query: HashMap<String, String>,
}

impl PageContext {
    pub fn new(raw_params: &RawPathParams, raw_query: Option<&str>) -> Self {
        let mut params = HashMap::new();
        for (key, value) in raw_params.iter() {
            params.insert(key.to_string(), value.to_string());
        }

        let query = raw_query
            .and_then(|q| serde_urlencoded::from_str::<HashMap<String, String>>(q).ok())
            .unwrap_or_default();

        Self { params, query }
    }

    /// A path parameter, e.g. `{repo}` in `/repo/{repo}/tree/{*path}`.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(String::as_str)
    }

    /// A path parameter, or `""` if absent (handy for optional wildcards).
    pub fn param_or_empty(&self, key: &str) -> &str {
        self.param(key).unwrap_or("")
    }

    /// A query-string value, e.g. `?q=foo`.
    pub fn query(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(String::as_str)
    }
}
