//! Global search over board titles and card text (live rows only).

use std::collections::HashMap;

use rusqlite::Connection;

use super::{boards, text};
use crate::models::SearchResult;
use crate::Result;

const MAX_QUERY: usize = 100;

fn like_pattern(q: &str) -> String {
    let mut p = String::with_capacity(q.len() + 2);
    p.push('%');
    for ch in q.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            p.push('\\');
        }
        p.push(ch);
    }
    p.push('%');
    p
}

pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let q: String = query.trim().chars().take(MAX_QUERY).collect();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let pat = like_pattern(&q.to_lowercase());
    let mut trails: HashMap<String, Vec<String>> = HashMap::new();
    let mut trail = |conn: &Connection, id: &str| -> Result<Vec<String>> {
        if let Some(t) = trails.get(id) {
            return Ok(t.clone());
        }
        let t: Vec<String> = boards::breadcrumb(conn, id)?.into_iter().map(|c| c.title).collect();
        trails.insert(id.to_string(), t.clone());
        Ok(t)
    };

    let mut out = Vec::new();
    let board_hits: Vec<(String, String)> = conn
        .prepare_cached(
            "SELECT id, title FROM boards WHERE trashed_by IS NULL AND lower(title) LIKE ?1 ESCAPE '\\' \
             ORDER BY updated_at DESC LIMIT ?2",
        )?
        .query_map(rusqlite::params![pat, limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, title) in board_hits {
        out.push(SearchResult {
            kind: "board".into(),
            path: trail(conn, &id)?,
            board_id: id.clone(),
            id,
            card_kind: None,
            title,
        });
    }
    let card_hits: Vec<(String, String, String, String)> = conn
        .prepare_cached(
            "SELECT c.id, c.board_id, c.kind, c.data FROM cards c JOIN boards b ON b.id = c.board_id \
             WHERE c.trashed_by IS NULL AND b.trashed_by IS NULL AND c.search_text LIKE ?1 ESCAPE '\\' \
             ORDER BY instr(c.search_text, ?3), length(c.search_text), c.updated_at DESC LIMIT ?2",
        )?
        .query_map(rusqlite::params![pat, limit as i64, q.to_lowercase()], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    for (id, board_id, kind, data) in card_hits {
        let data: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
        out.push(SearchResult {
            kind: "card".into(),
            title: text::summary(&kind, &data, Some(&q)),
            path: trail(conn, &board_id)?,
            id,
            board_id,
            card_kind: Some(kind),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::like_pattern;

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(like_pattern("50%_off\\"), "%50\\%\\_off\\\\%");
    }
}
