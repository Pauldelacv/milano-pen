//! Board exports.
//!
//! Markdown follows the shape of Milaclone's exporter (MIT, (c) 2026 ellshep)
//! so it stays readable by its Milanote-Markdown importer: `# board` sections,
//! `## first line` + lines for notes, `- [ ]` tasks for to-dos, `[title](url)`
//! for links and files. Nested boards become further `#` sections.
//!
//! JSON is a lossless [`Snapshot`] of the board's cards and nested boards,
//! importable with [`super::workspace::Workspace::import_json`].

use rusqlite::Connection;
use serde_json::Value;

use super::cards::{self, Snapshot};
use crate::models::{BoardRow, CardRow};
use crate::{Error, Result};

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn label(s: &str) -> String {
    one_line(s).replace(['[', ']'], "")
}

fn url(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "%28".to_string(),
            ')' => "%29".to_string(),
            c if c.is_whitespace() => "%20".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn s<'a>(d: &'a Value, k: &str) -> &'a str {
    d.get(k).and_then(Value::as_str).unwrap_or_default()
}

fn card_lines(c: &CardRow) -> Vec<String> {
    let d = &c.data;
    match c.kind.as_str() {
        "note" | "comment" | "swatch" | "table" | "document" => {
            let mut lines: Vec<String> =
                s(d, "text").lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
            if c.kind == "swatch" && lines.is_empty() && !s(d, "hex").is_empty() {
                lines.push(s(d, "hex").to_string());
            }
            if c.kind == "document" {
                let t = one_line(s(d, "title"));
                lines.insert(0, if t.is_empty() { "Document".into() } else { t });
            }
            if lines.is_empty() {
                return vec![];
            }
            let mut out = vec![format!("## {}", lines[0])];
            out.extend(lines[1..].iter().cloned());
            out.push(String::new());
            out
        }
        "todo" => {
            let tasks: Vec<String> = d
                .get("tasks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|t| {
                    let text = one_line(t.get("text")?.as_str()?);
                    if text.is_empty() {
                        return None;
                    }
                    let done = t.get("done").and_then(Value::as_bool).unwrap_or(false);
                    Some(format!("- [{}] {text}", if done { 'x' } else { ' ' }))
                })
                .collect();
            if tasks.is_empty() {
                return vec![];
            }
            let title = one_line(s(d, "title"));
            let mut out = vec![format!("## {}", if title.is_empty() { "To-do" } else { &title })];
            out.extend(tasks);
            out.push(String::new());
            out
        }
        "link" if !s(d, "url").is_empty() => {
            let t = label(s(d, "title"));
            let t = if t.is_empty() { label(s(d, "url")) } else { t };
            vec![format!("[{t}]({})", url(s(d, "url"))), String::new()]
        }
        "image" | "file" => {
            let name = label(if s(d, "caption").is_empty() { s(d, "name") } else { s(d, "caption") });
            let name = if name.is_empty() { "file".into() } else { name };
            let bang = if c.kind == "image" { "!" } else { "" };
            vec![format!("{bang}[{name}](assets/{})", url(s(d, "name"))), String::new()]
        }
        _ => vec![],
    }
}

pub fn markdown(conn: &Connection, board_id: &str) -> Result<String> {
    BoardRow::get_live(conn, board_id)?.ok_or_else(|| Error::NotFound(format!("board {board_id}")))?;
    let mut out: Vec<String> = Vec::new();
    let mut queue = vec![board_id.to_string()];
    let mut i = 0;
    while i < queue.len() {
        let Some(board) = BoardRow::get_live(conn, &queue[i])? else {
            i += 1;
            continue;
        };
        i += 1;
        out.push(format!("# {}", one_line(&board.title)));
        out.push(String::new());
        let all = CardRow::query(conn, "board_id = ?1 AND trashed_by IS NULL", [&board.id])?;
        let mut free: Vec<&CardRow> = all.iter().filter(|c| c.parent_card_id.is_none()).collect();
        free.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        for c in free {
            if let Some(child) = c.child_board_id() {
                queue.push(child.to_string());
            }
            if c.kind == "column" {
                let title = one_line(s(&c.data, "title"));
                if !title.is_empty() {
                    out.push(format!("## {title}"));
                    out.push(String::new());
                }
                let mut kids: Vec<&CardRow> =
                    all.iter().filter(|k| k.parent_card_id.as_deref() == Some(&c.id)).collect();
                kids.sort_by(|a, b| a.y.total_cmp(&b.y));
                for k in kids {
                    out.extend(card_lines(k));
                }
            } else {
                out.extend(card_lines(c));
            }
        }
    }
    let mut md = out.join("\n");
    while md.contains("\n\n\n") {
        md = md.replace("\n\n\n", "\n\n");
    }
    Ok(md.trim_end().to_string() + "\n")
}

/// Every card on the board (with nested boards) as a snapshot.
pub fn json(conn: &Connection, board_id: &str) -> Result<Snapshot> {
    BoardRow::get_live(conn, board_id)?.ok_or_else(|| Error::NotFound(format!("board {board_id}")))?;
    let ids: Vec<String> =
        CardRow::query(conn, "board_id = ?1 AND trashed_by IS NULL AND parent_card_id IS NULL", [board_id])?
            .into_iter()
            .map(|c| c.id)
            .collect();
    cards::snapshot(conn, &ids)
}
