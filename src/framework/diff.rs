use maud::Markup;

/// A patch representing a changed element identified by its `id` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub id: String,
    pub html: String,
}

/// Computes the minimal set of patches needed to transform `old` Markup into `new` Markup.
///
/// Patches are identified by element IDs. The algorithm finds the deepest (most specific)
/// changed elements and returns their new outer HTML. If changes exist outside of ID'd
/// sub-elements, the nearest ancestor with an ID is returned instead.
pub fn diff(old: &Markup, new: &Markup) -> Vec<Patch> {
    let old_html = old.clone().into_string();
    let new_html = new.clone().into_string();

    if old_html == new_html {
        return vec![];
    }

    let old_doc = scraper::Html::parse_fragment(&old_html);
    let new_doc = scraper::Html::parse_fragment(&new_html);

    let id_selector = scraper::Selector::parse("[id]").unwrap();

    let old_root = match old_doc.select(&id_selector).next() {
        Some(el) => el,
        None => return vec![],
    };
    let new_root = match new_doc.select(&id_selector).next() {
        Some(el) => el,
        None => return vec![],
    };

    let mut patches = Vec::new();
    diff_element(&old_doc, old_root, &new_doc, new_root, &mut patches);
    patches
}

fn diff_element(
    old_doc: &scraper::Html,
    old_el: scraper::ElementRef,
    new_doc: &scraper::Html,
    new_el: scraper::ElementRef,
    patches: &mut Vec<Patch>,
) {
    let old_html = old_el.html();
    let new_html = new_el.html();

    if old_html == new_html {
        return;
    }

    let id_selector = scraper::Selector::parse("[id]").unwrap();

    // Collect direct ID'd descendants (not self, and not nested under another ID'd descendant)
    let child_ids: Vec<String> = new_el
        .select(&id_selector)
        .filter(|el| {
            if el.value().id() == new_el.value().id() {
                return false;
            }
            let mut parent = el.parent().and_then(scraper::ElementRef::wrap);
            while let Some(p) = parent {
                if p.value().id().is_some() {
                    return p.value().id() == new_el.value().id();
                }
                parent = p.parent().and_then(scraper::ElementRef::wrap);
            }
            false
        })
        .filter_map(|el| el.value().id().map(|id| id.to_string()))
        .collect();

    // Recursively diff each ID'd child
    let mut child_patches = Vec::new();
    let mut old_child_htmls: Vec<(String, String)> = Vec::new();

    for child_id in &child_ids {
        let selector_str = format!("#{}", child_id);
        let sel = scraper::Selector::parse(&selector_str).unwrap();

        let old_child = old_doc.select(&sel).next();
        let new_child = new_doc.select(&sel).next();

        if let (Some(oc), Some(nc)) = (old_child, new_child) {
            old_child_htmls.push((oc.html(), nc.html()));
            diff_element(old_doc, oc, new_doc, nc, &mut child_patches);
        }
    }

    if child_patches.is_empty() && child_ids.is_empty() {
        if let Some(id) = new_el.value().id() {
            patches.push(Patch {
                id: id.to_string(),
                html: new_html,
            });
        }
        return;
    }

    // Check if child patches fully cover the changes
    let mut simulated = old_html.clone();
    for (old_child_html, new_child_html) in &old_child_htmls {
        simulated = simulated.replacen(old_child_html, new_child_html, 1);
    }

    if simulated == new_html {
        patches.extend(child_patches);
    } else {
        if let Some(id) = new_el.value().id() {
            patches.push(Patch {
                id: id.to_string(),
                html: new_html,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maud::html;

    #[test]
    fn no_change_returns_empty() {
        let markup = html! { div id="root" { p id="count" { "count: 5" } } };
        assert_eq!(diff(&markup, &markup), vec![]);
    }

    #[test]
    fn leaf_id_element_changed() {
        let old = html! { div id="root" { p id="count" { "count: 5" } div id="buttons" { button { "+1" } } } };
        let new = html! { div id="root" { p id="count" { "count: 6" } div id="buttons" { button { "+1" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "count");
        assert!(patches[0].html.contains("count: 6"));
    }

    #[test]
    fn non_id_element_changed_falls_back_to_parent() {
        let old = html! { div id="root" { p id="count" { "count: 5" } span { "old" } } };
        let new = html! { div id="root" { p id="count" { "count: 6" } span { "new" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "root");
    }

    #[test]
    fn multiple_leaf_changes() {
        let old = html! { div id="root" { p id="a" { "old-a" } p id="b" { "old-b" } } };
        let new = html! { div id="root" { p id="a" { "new-a" } p id="b" { "new-b" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 2);
        let ids: Vec<&str> = patches.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn only_non_id_element_changed() {
        let old = html! { div id="root" { span { "old" } } };
        let new = html! { div id="root" { span { "new" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "root");
    }

    #[test]
    fn deeply_nested_id_change() {
        let old = html! { div id="root" { div id="mid" { p id="leaf" { "old" } } } };
        let new = html! { div id="root" { div id="mid" { p id="leaf" { "new" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "leaf");
    }

    #[test]
    fn no_id_elements_returns_empty() {
        let old = html! { div { span { "old" } } };
        let new = html! { div { span { "new" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches, vec![]);
    }

    // --- 属性の変更 ---

    #[test]
    fn attribute_change_detected() {
        let old = html! { div id="root" { a id="link" href="old.html" { "click" } } };
        let new = html! { div id="root" { a id="link" href="new.html" { "click" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "link");
        assert!(patches[0].html.contains("new.html"));
    }

    #[test]
    fn class_change_detected() {
        let old = html! { div id="root" { div id="box" class="red" { "hello" } } };
        let new = html! { div id="root" { div id="box" class="blue" { "hello" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "box");
    }

    // --- 部分変更 ---

    #[test]
    fn one_of_multiple_siblings_changed() {
        let old = html! { div id="root" { p id="a" { "same" } p id="b" { "old" } p id="c" { "same" } } };
        let new = html! { div id="root" { p id="a" { "same" } p id="b" { "new" } p id="c" { "same" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "b");
    }

    // --- 構造変更 ---

    #[test]
    fn child_element_added() {
        let old = html! { div id="root" { ul id="list" { li { "item1" } } } };
        let new = html! { div id="root" { ul id="list" { li { "item1" } li { "item2" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "list");
    }

    #[test]
    fn child_element_removed() {
        let old = html! { div id="root" { ul id="list" { li { "item1" } li { "item2" } } } };
        let new = html! { div id="root" { ul id="list" { li { "item1" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "list");
    }

    #[test]
    fn element_tag_changed() {
        let old = html! { div id="root" { div id="content" { p { "text" } } } };
        let new = html! { div id="root" { div id="content" { span { "text" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "content");
    }

    // --- カバレッジ境界 ---

    #[test]
    fn id_child_changed_plus_non_id_sibling_changed_falls_back_to_parent() {
        // #count changed AND <span> (no id) changed → parent #root must be sent
        let old = html! { div id="root" { p id="count" { "5" } span { "status: ok" } } };
        let new = html! { div id="root" { p id="count" { "6" } span { "status: error" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "root");
    }

    #[test]
    fn id_child_unchanged_but_non_id_sibling_changed() {
        let old = html! { div id="root" { p id="count" { "5" } span { "old" } } };
        let new = html! { div id="root" { p id="count" { "5" } span { "new" } } };
        let patches = diff(&old, &new);
        // #count is same, but <span> changed → simulated won't match → root
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "root");
    }

    // --- 深いネスト ---

    #[test]
    fn change_at_middle_level_not_leaf() {
        // mid has id and changes, but leaf (with id) does NOT change
        let old = html! { div id="root" { div id="mid" { span { "old" } p id="leaf" { "same" } } } };
        let new = html! { div id="root" { div id="mid" { span { "new" } p id="leaf" { "same" } } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "mid");
    }

    #[test]
    fn changes_at_multiple_nesting_levels() {
        // Both mid-level and leaf change, but leaf covers its own change
        let old = html! { div id="root" { div id="mid" { p id="leaf" { "old" } } span { "same" } } };
        let new = html! { div id="root" { div id="mid" { p id="leaf" { "new" } } span { "same" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "leaf");
    }

    // --- 空コンテンツ ---

    #[test]
    fn empty_to_content() {
        let old = html! { div id="root" { p id="msg" {} } };
        let new = html! { div id="root" { p id="msg" { "hello" } } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "msg");
    }

    #[test]
    fn content_to_empty() {
        let old = html! { div id="root" { p id="msg" { "hello" } } };
        let new = html! { div id="root" { p id="msg" {} } };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "msg");
    }

    // --- 実アプリケーションに近いシナリオ ---

    #[test]
    fn counter_app_increment() {
        let old = html! {
            div id="maudliver-root" {
                p id="count" { "count: 0" }
                div id="buttons" {
                    button data-event="Increment" { "+1" }
                    " "
                    button data-event="Decrement" { "-1" }
                }
            }
        };
        let new = html! {
            div id="maudliver-root" {
                p id="count" { "count: 1" }
                div id="buttons" {
                    button data-event="Increment" { "+1" }
                    " "
                    button data-event="Decrement" { "-1" }
                }
            }
        };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].id, "count");
        assert!(patches[0].html.contains("count: 1"));
        // buttons should NOT be in patches
        assert!(!patches.iter().any(|p| p.id == "buttons"));
    }

    #[test]
    fn todo_list_add_item() {
        let old = html! {
            div id="maudliver-root" {
                h1 id="title" { "TODOs" }
                ul id="items" {
                    li { "Buy milk" }
                }
                p id="status" { "1 item" }
            }
        };
        let new = html! {
            div id="maudliver-root" {
                h1 id="title" { "TODOs" }
                ul id="items" {
                    li { "Buy milk" }
                    li { "Walk dog" }
                }
                p id="status" { "2 items" }
            }
        };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 2);
        let ids: Vec<&str> = patches.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"items"));
        assert!(ids.contains(&"status"));
        assert!(!ids.contains(&"title"));
    }

    #[test]
    fn conditional_visibility_toggle() {
        let old = html! {
            div id="maudliver-root" {
                div id="modal" style="display:none" { "hidden content" }
                button id="trigger" { "Show" }
            }
        };
        let new = html! {
            div id="maudliver-root" {
                div id="modal" style="display:block" { "hidden content" }
                button id="trigger" { "Hide" }
            }
        };
        let patches = diff(&old, &new);
        assert_eq!(patches.len(), 2);
        let ids: Vec<&str> = patches.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"modal"));
        assert!(ids.contains(&"trigger"));
    }
}
