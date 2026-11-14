//! Plain text derived from a card's data, for search, the trash list and
//! exports. Rich-text cards carry their own plain text in `data.text`
//! (written by the editor), so no rich-text parsing happens here.

use serde_json::Value;

const TEXT_FIELDS: &[&str] = &["title", "text", "caption", "name", "url", "description", "hex"];

/// Every human-readable string of a card, one per line.
pub fn plain_lines(data: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in TEXT_FIELDS {
        if let Some(s) = data.get(*key).and_then(Value::as_str) {
            out.extend(s.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from));
        }
    }
    if let Some(tasks) = data.get("tasks").and_then(Value::as_array) {
        for t in tasks {
            if let Some(s) = t.get("text").and_then(Value::as_str) {
                let s = s.trim();
                if !s.is_empty() {
                    out.push(s.to_string());
                }
            }
        }
    }
    if let Some(labels) = data.get("labels").and_then(Value::as_array) {
        out.extend(labels.iter().filter_map(Value::as_str).map(String::from));
    }
    out
}

/// Lower-cased text stored in `cards.search_text`.
pub fn search_text(kind: &str, data: &Value) -> String {
    if kind == "line" {
        return data.get("label").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    }
    plain_lines(data).join("\n").to_lowercase()
}

/// A one-line title for lists (trash, search results).
pub fn summary(kind: &str, data: &Value, query: Option<&str>) -> String {
    let lines = plain_lines(data);
    let hit = query.map(str::to_lowercase).and_then(|q| lines.iter().find(|l| l.to_lowercase().contains(&q)).cloned());
    let line = hit.or_else(|| lines.first().cloned()).unwrap_or_else(|| default_title(kind).to_string());
    truncate(&line, 140)
}

pub fn default_title(kind: &str) -> &'static str {
    match kind {
        "note" => "Note",
        "document" => "Document",
        "todo" => "To-do",
        "image" => "Image",
        "file" => "File",
        "link" => "Link",
        "column" => "Column",
        "board" => "Board",
        "table" => "Table",
        "comment" => "Comment",
        "swatch" => "Colour",
        "sketch" => "Sketch",
        "line" => "Line",
        _ => "Card",
    }
}

pub fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max_chars - 1).collect();
    t.push('…');
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collects_text_tasks_and_labels() {
        let data = json!({
            "title": "Groceries",
            "tasks": [{"text": "Milk"}, {"text": "  "}, {"text": "Eggs"}],
            "labels": ["Home"],
        });
        assert_eq!(search_text("todo", &data), "groceries\nmilk\neggs\nhome");
    }

    #[test]
    fn summary_prefers_the_matching_line() {
        let data = json!({"text": "first line\nsecond Match here"});
        assert_eq!(summary("note", &data, Some("match")), "second Match here");
        assert_eq!(summary("note", &data, None), "first line");
        assert_eq!(summary("note", &json!({}), None), "Note");
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate("héllo wörld", 6), "héllo…");
        assert_eq!(truncate("short", 10), "short");
    }
}
