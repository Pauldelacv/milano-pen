//! A synthetic large board for performance testing: ≥ 1,000 cards including
//! ≥ 300 distinct images, columns with cards inside, nested boards and lines.

use image::{Rgb, RgbImage};
use serde_json::json;

use super::cards;
use super::history::Recorder;
use super::workspace::Workspace;
use crate::files::AssetStore;
use crate::models::{NewCard, Patch};
use crate::Result;

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkSpec {
    pub notes: usize,
    pub images: usize,
    pub columns: usize,
    pub cards_per_column: usize,
    pub boards: usize,
    pub lines: usize,
    pub image_size: (u32, u32),
}

impl Default for BenchmarkSpec {
    fn default() -> Self {
        Self {
            notes: 520,
            images: 300,
            columns: 20,
            cards_per_column: 5,
            boards: 40,
            lines: 60,
            image_size: (1600, 1200),
        }
    }
}

impl BenchmarkSpec {
    pub fn total_cards(&self) -> usize {
        self.notes + self.images + self.columns * (1 + self.cards_per_column) + self.boards + self.lines
    }
}

/// A distinct JPEG per index (gradient with a varying hue).
pub fn synthetic_jpeg(i: usize, (w, h): (u32, u32)) -> Vec<u8> {
    let hue = (i * 47 % 360) as f32;
    let (r0, g0, b0) = hsv(hue, 0.55, 0.95);
    let (r1, g1, b1) = hsv((hue + 60.0) % 360.0, 0.7, 0.55);
    let img = RgbImage::from_fn(w, h, |x, y| {
        let t = (x as f32 / w as f32 + y as f32 / h as f32) / 2.0;
        let mix = |a: f32, b: f32| (a + (b - a) * t) as u8;
        let stripe = if (x / 40 + y / 40 + i as u32).is_multiple_of(7) { 25 } else { 0 };
        Rgb([mix(r0, r1).saturating_add(stripe), mix(g0, g1), mix(b0, b1)])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .encode_image(&img)
        .expect("in-memory JPEG encoding");
    out
}

fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    ((r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0)
}

/// Encode every benchmark image, in parallel. Run this before [`generate`]
/// and outside any lock: it is the slow part.
pub fn synthetic_images(spec: &BenchmarkSpec) -> Vec<Vec<u8>> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).max(1);
    let mut out: Vec<Vec<u8>> = vec![Vec::new(); spec.images];
    let chunk = spec.images.div_ceil(threads).max(1);
    std::thread::scope(|s| {
        for (c, slots) in out.chunks_mut(chunk).enumerate() {
            let size = spec.image_size;
            s.spawn(move || {
                for (j, slot) in slots.iter_mut().enumerate() {
                    *slot = synthetic_jpeg(c * chunk + j, size);
                }
            });
        }
    });
    out
}

/// Create the benchmark board on `parent_board_id` from pre-encoded `images`
/// (see [`synthetic_images`]). Returns the patch (with the new board card)
/// and the image asset ids, for thumbnail prefetch.
pub fn generate(
    ws: &mut Workspace,
    parent_board_id: &str,
    spec: BenchmarkSpec,
    images: &[Vec<u8>],
) -> Result<(Patch, Vec<String>)> {
    let mut asset_ids = Vec::new();
    let patch = ws.batch("Generate benchmark board", |tx, rec: &mut Recorder, store: Option<&AssetStore>| {
        let board_card = cards::create(
            tx,
            rec,
            &NewCard {
                board_id: parent_board_id.into(),
                kind: "board".into(),
                x: 40.0,
                y: 40.0,
                data: Some(json!({ "title": format!("Benchmark ({} cards)", spec.total_cards()), "icon": "zap" })),
                color: Some("orange".into()),
                ..Default::default()
            },
        )?;
        let board = crate::models::CardRow::get_live(tx, &board_card)?
            .and_then(|c| c.child_board_id().map(String::from))
            .expect("board card has a child board");

        let cols = 40usize;
        let cell = (300.0, 280.0);
        let mut slot = 0usize;
        let mut next_pos = || {
            let p = ((slot % cols) as f64 * cell.0, (slot / cols) as f64 * cell.1);
            slot += 1;
            p
        };
        let mut placed: Vec<String> = Vec::new();
        let mk = |kind: &str, (x, y): (f64, f64), data: serde_json::Value| NewCard {
            board_id: board.clone(),
            kind: kind.into(),
            x,
            y,
            data: Some(data),
            ..Default::default()
        };

        for (i, bytes) in images.iter().enumerate() {
            let pos = next_pos();
            let Some(store) = store else { break };
            let asset = store.import_bytes(tx, &format!("benchmark-{i:03}.jpg"), bytes)?;
            asset_ids.push(asset.id.clone());
            let (iw, ih) = (spec.image_size.0 as f64, spec.image_size.1 as f64);
            let mut n = mk("image", pos, json!({
                "assetId": asset.id, "name": asset.original_name, "width": iw, "height": ih, "mime": asset.mime,
            }));
            n.w = Some(240.0);
            n.h = Some(240.0 * ih / iw);
            placed.push(cards::create(tx, rec, &n)?);
        }
        for i in 0..spec.notes {
            let text = format!("Note {i}\nLorem ipsum dolor sit amet, consectetur adipiscing elit.");
            let doc = json!({"type": "doc", "content": [
                {"type": "paragraph", "content": [{"type": "text", "marks": [{"type": "bold"}], "text": format!("Note {i}")}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "Lorem ipsum dolor sit amet, consectetur adipiscing elit."}]}
            ]});
            placed.push(cards::create(tx, rec, &mk("note", next_pos(), json!({ "text": text, "doc": doc })))?);
        }
        for i in 0..spec.columns {
            let pos = next_pos();
            let col = cards::create(tx, rec, &mk("column", pos, json!({ "title": format!("Column {i}") })))?;
            for j in 0..spec.cards_per_column {
                let mut n = mk("note", (0.0, j as f64 * 100.0), json!({ "text": format!("Column {i} · card {j}") }));
                n.parent_card_id = Some(col.clone());
                cards::create(tx, rec, &n)?;
            }
            placed.push(col);
        }
        for i in 0..spec.boards {
            let id = cards::create(tx, rec, &mk("board", next_pos(), json!({ "title": format!("Nested board {i}") })))?;
            let child = crate::models::CardRow::get_live(tx, &id)?
                .and_then(|c| c.child_board_id().map(String::from))
                .expect("child board");
            for j in 0..2 {
                let n = NewCard {
                    board_id: child.clone(),
                    kind: "note".into(),
                    x: 40.0 + j as f64 * 280.0,
                    y: 40.0,
                    data: Some(json!({ "text": format!("Inside board {i} · {j}") })),
                    ..Default::default()
                };
                cards::create(tx, rec, &n)?;
            }
            placed.push(id);
        }
        for i in 0..spec.lines.min(placed.len().saturating_sub(1)) {
            let (a, b) = (&placed[i * 7 % placed.len()], &placed[(i * 7 + 1) % placed.len()]);
            cards::create(tx, rec, &mk("line", (0.0, 0.0), json!({ "from": {"id": a}, "to": {"id": b}, "arrow": "end" })))?;
        }
        Ok(vec![board_card])
    })?;
    Ok((patch, asset_ids))
}
