//! End-to-end tests of the core through the `Workspace` API the app uses.

use std::time::Instant;

use milano_core::models::{BoardUpdate, CardUpdate, NewCard};
use milano_core::services::benchmark::{self, BenchmarkSpec};
use milano_core::{AppPaths, Workspace};
use serde_json::json;

fn note(board: &str, x: f64, y: f64, text: &str) -> NewCard {
    NewCard {
        board_id: board.into(),
        kind: "note".into(),
        x,
        y,
        data: Some(json!({ "text": text })),
        ..Default::default()
    }
}

fn board_card(board: &str, title: &str) -> NewCard {
    NewCard {
        board_id: board.into(),
        kind: "board".into(),
        x: 0.0,
        y: 0.0,
        data: Some(json!({ "title": title })),
        ..Default::default()
    }
}

#[test]
fn creates_moves_and_persists_across_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path());
    let (root, id) = {
        let mut ws = Workspace::open(&paths).unwrap();
        let root = ws.root_board_id().unwrap();
        let p = ws.create_cards(&[note(&root, 10.0, 20.0, "hello")]).unwrap();
        assert_eq!(p.created.len(), 1);
        let id = p.created[0].clone();
        let p = ws
            .update_cards(
                &[CardUpdate { id: id.clone(), x: Some(300.0), y: Some(400.0), w: Some(5.0), ..Default::default() }],
                "Move",
                None,
            )
            .unwrap();
        assert_eq!(p.cards[0].x, 300.0);
        assert_eq!(p.cards[0].w, 24.0, "width is clamped to the minimum");
        (root, id)
    };
    let ws = Workspace::open(&paths).unwrap();
    assert_eq!(ws.root_board_id().unwrap(), root, "the Home board survives a restart");
    let view = ws.open_board(&root).unwrap();
    assert_eq!(view.board.title, "Home");
    let card = view.cards.iter().find(|c| c.id == id).unwrap();
    assert_eq!((card.x, card.y), (300.0, 400.0));
    assert!(paths.database_file().exists());
}

#[test]
fn rejects_bad_input() {
    let mut ws = Workspace::in_memory().unwrap();
    let root = ws.root_board_id().unwrap();
    assert!(ws.create_cards(&[NewCard { kind: "rocket".into(), ..note(&root, 0.0, 0.0, "") }]).is_err());
    assert!(ws.create_cards(&[note("missing", 0.0, 0.0, "")]).is_err());
    assert!(ws.create_cards(&[note(&root, f64::NAN, 0.0, "")]).is_err());
    let id = ws.create_cards(&[note(&root, 0.0, 0.0, "x")]).unwrap().created[0].clone();
    let mut bad_parent = note(&root, 0.0, 0.0, "y");
    bad_parent.parent_card_id = Some(id); // a note is not a column
    assert!(ws.create_cards(&[bad_parent]).is_err());
    // A failed operation leaves nothing behind and records no history.
    assert_eq!(ws.open_board(&root).unwrap().cards.len(), 1);
}

#[test]
fn undo_and_redo_cover_create_edit_and_delete() {
    let mut ws = Workspace::in_memory().unwrap();
    let root = ws.root_board_id().unwrap();
    let id = ws.create_cards(&[note(&root, 0.0, 0.0, "a")]).unwrap().created[0].clone();
    // Typing in one note within the coalescing window is one undo step.
    for t in ["ab", "abc", "abcd"] {
        ws.update_cards(
            &[CardUpdate { id: id.clone(), data: Some(json!({ "text": t })), ..Default::default() }],
            "Edit note",
            Some(format!("edit:{id}")),
        )
        .unwrap();
    }
    ws.trash_cards(std::slice::from_ref(&id)).unwrap();
    assert!(ws.open_board(&root).unwrap().cards.is_empty());

    let p = ws.undo().unwrap().unwrap(); // un-delete
    assert_eq!(p.cards[0].data["text"], "abcd");
    let p = ws.undo().unwrap().unwrap(); // un-edit (all three keystrokes)
    assert_eq!(p.cards[0].data["text"], "a");
    let p = ws.undo().unwrap().unwrap(); // un-create
    assert_eq!(p.removed_cards, vec![id.clone()]);
    assert!(ws.undo().unwrap().is_none());
    assert!(ws.history_state().can_redo);

    ws.redo().unwrap();
    let p = ws.redo().unwrap().unwrap();
    assert_eq!(p.cards[0].data["text"], "abcd");
    // A new edit clears the redo stack.
    ws.update_cards(&[CardUpdate { id: id.clone(), x: Some(5.0), ..Default::default() }], "Move", None).unwrap();
    assert!(!ws.history_state().can_redo);
    // A no-op change records nothing.
    ws.update_cards(&[CardUpdate { id: id.clone(), x: Some(5.0), ..Default::default() }], "Move", None).unwrap();
    assert_eq!(ws.history_state().undo_label.as_deref(), Some("Move"));
}

#[test]
fn nested_boards_trash_restore_and_copy_deeply() {
    let mut ws = Workspace::in_memory().unwrap();
    let root = ws.root_board_id().unwrap();
    let p = ws.create_cards(&[board_card(&root, "Trip")]).unwrap();
    let child = p.cards[0].data["childBoardId"].as_str().unwrap().to_string();
    assert!(p.boards.iter().any(|b| b.id == child && b.title == "Trip"));
    let bid = p.created[0].clone();
    ws.create_cards(&[note(&child, 0.0, 0.0, "pack bags")]).unwrap();
    let grand = ws.create_cards(&[board_card(&child, "Hotels")]).unwrap();
    let grand_board = grand.cards[0].data["childBoardId"].as_str().unwrap().to_string();
    ws.create_cards(&[note(&grand_board, 0.0, 0.0, "deep")]).unwrap();

    // Title/colour of a board card are written through to the board.
    let p = ws
        .update_cards(
            &[CardUpdate {
                id: bid.clone(),
                color: Some("teal".into()),
                data: Some(json!({"title": "Holiday"})),
                ..Default::default()
            }],
            "Edit",
            None,
        )
        .unwrap();
    let b = p.boards.iter().find(|b| b.id == child).unwrap();
    assert_eq!((b.title.as_str(), b.color.as_deref()), ("Holiday", Some("teal")));
    assert!(p.cards[0].data.get("title").is_none(), "title is not duplicated on the card");
    let crumbs = ws.open_board(&grand_board).unwrap().breadcrumb;
    assert_eq!(crumbs.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), ["Home", "Holiday", "Hotels"]);

    // Duplicate copies the whole tree with new ids.
    let p = ws.duplicate(std::slice::from_ref(&bid)).unwrap();
    let copy = p.cards.iter().find(|c| p.created.contains(&c.id)).unwrap();
    let copy_board = copy.data["childBoardId"].as_str().unwrap();
    assert_ne!(copy_board, child);
    assert_eq!((copy.x, copy.y), (24.0, 24.0));
    let inner = ws.open_board(copy_board).unwrap();
    assert_eq!(inner.cards.len(), 2);
    assert_eq!(inner.child_boards.len(), 1);
    assert_ne!(inner.child_boards[0].id, grand_board);

    // Trashing the board card trashes everything inside; search ignores it.
    ws.trash_cards(std::slice::from_ref(&bid)).unwrap();
    assert!(ws.open_board(&child).is_err());
    assert!(ws.search("deep").unwrap().iter().all(|r| r.board_id != grand_board));
    let trash = ws.trash_list().unwrap();
    assert_eq!(trash.len(), 1, "one entry per delete");
    assert_eq!(trash[0].title, "Holiday");
    ws.restore_trashed(&bid).unwrap();
    assert_eq!(ws.open_board(&grand_board).unwrap().cards.len(), 1);
}

#[test]
fn copy_paste_keeps_layout_and_z_order() {
    let mut ws = Workspace::in_memory().unwrap();
    let root = ws.root_board_id().unwrap();
    let p = ws.create_cards(&[note(&root, 100.0, 100.0, "a"), note(&root, 300.0, 150.0, "b")]).unwrap();
    assert_eq!(ws.copy(&p.created).unwrap(), 2);
    assert!(ws.has_clipboard());
    ws.trash_cards(&p.created).unwrap(); // the clipboard survives the delete
    let pasted = ws.paste(&root, Some((1000.0, 1000.0))).unwrap();
    let mut pos: Vec<(f64, f64)> = pasted.cards.iter().map(|c| (c.x, c.y)).collect();
    pos.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(pos, vec![(1000.0, 1000.0), (1200.0, 1050.0)]);

    let ids = pasted.created.clone();
    let p = ws.restack(&ids[..1], false).unwrap();
    let z_back = p.cards[0].z;
    let all = ws.open_board(&root).unwrap().cards;
    assert!(all.iter().all(|c| c.z >= z_back));
    let p = ws.restack(&ids[..1], true).unwrap();
    assert!(all.iter().all(|c| c.z <= p.cards[0].z));
}

#[test]
fn search_export_and_json_round_trip() {
    let mut ws = Workspace::in_memory().unwrap();
    let root = ws.root_board_id().unwrap();
    ws.update_board(&root, &BoardUpdate { title: Some("  My  Home ".into()), ..Default::default() }).unwrap();
    ws.create_cards(&[
        note(&root, 0.0, 0.0, "Buy Milk\nand bread"),
        NewCard { board_id: root.clone(), kind: "todo".into(), x: 0.0, y: 200.0,
                  data: Some(json!({"title": "Today", "tasks": [{"id": "1", "text": "Call Ana", "done": true}, {"id": "2", "text": "100% done_"}]})), ..Default::default() },
        NewCard { board_id: root.clone(), kind: "link".into(), x: 0.0, y: 400.0,
                  data: Some(json!({"url": "https://example.com/a b", "title": "Example"})), ..Default::default() },
    ])
    .unwrap();
    let hits = ws.search("milk").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Buy Milk");
    assert_eq!(hits[0].path, vec!["My  Home".to_string()]);
    assert_eq!(ws.search("%").unwrap().len(), 1, "wildcards are literal");
    assert_eq!(ws.search("home").unwrap().len(), 1);

    let md = ws.export_markdown(&root).unwrap();
    assert!(md.starts_with("# My Home\n"));
    assert!(md.contains("## Buy Milk\nand bread\n"));
    assert!(md.contains("## Today\n- [x] Call Ana\n- [ ] 100% done_\n"));
    assert!(md.contains("[Example](https://example.com/a%20b)"));

    let json = ws.export_json(&root).unwrap();
    let p = ws.create_cards(&[board_card(&root, "Imported")]).unwrap();
    let target = p.cards[0].data["childBoardId"].as_str().unwrap().to_string();
    let imported = ws.import_json(&target, &json, (0.0, 0.0)).unwrap();
    assert_eq!(imported.created.len(), 3);
    assert_eq!(ws.open_board(&target).unwrap().cards.len(), 3);
    assert!(ws.import_json(&target, "{\"nope\":1}", (0.0, 0.0)).is_err());

    ws.create_cards(&[note(&root, 0.0, 0.0, "note 424"), note(&root, 0.0, 0.0, "note 42")]).unwrap();
    assert_eq!(ws.search("note 42").unwrap()[0].title, "note 42", "the closest match ranks first");
}

#[test]
fn imports_files_dedupes_and_empties_trash() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().join("data"));
    let mut ws = Workspace::open(&paths).unwrap();
    let root = ws.root_board_id().unwrap();
    let img = dir.path().join("Photo.PNG");
    image::RgbImage::from_pixel(800, 400, image::Rgb([200, 10, 10]))
        .save_with_format(&img, image::ImageFormat::Png)
        .unwrap();
    let pdf = dir.path().join("doc.pdf");
    std::fs::write(&pdf, b"%PDF-1.4 fake").unwrap();
    let r = ws.import_files(&root, &[img.clone(), pdf, dir.path().join("missing.txt")], (0.0, 0.0)).unwrap();
    assert_eq!(r.patch.created.len(), 2);
    assert_eq!(r.errors.len(), 1);
    let image_card = r.patch.cards.iter().find(|c| c.kind == "image").unwrap();
    assert_eq!(image_card.h, Some(140.0), "aspect ratio known at import");
    let asset_id = image_card.data["assetId"].as_str().unwrap().to_string();
    let original = ws.asset_store().unwrap().original_path(&asset_id, "png");
    assert!(original.exists());
    assert_eq!(std::fs::read(&original).unwrap(), std::fs::read(&img).unwrap(), "original preserved");

    // Same content again: one asset, two cards.
    let again = ws.import_files(&root, &[img], (0.0, 500.0)).unwrap();
    assert_eq!(again.assets[0].id, asset_id);
    let count: i64 = ws.conn().query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);

    // The file stays while any card (even trashed) references it.
    ws.trash_cards(std::slice::from_ref(&image_card.id)).unwrap();
    ws.empty_trash().unwrap();
    assert!(original.exists());
    ws.trash_cards(&again.patch.created).unwrap();
    let report = ws.empty_trash().unwrap();
    assert_eq!(report.files, 1);
    assert!(!original.exists());
    assert!(!ws.history_state().can_undo, "emptying the trash clears history");
}

#[test]
fn backup_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().join("data"));
    let mut ws = Workspace::open(&paths).unwrap();
    let root = ws.root_board_id().unwrap();
    ws.create_cards(&[note(&root, 0.0, 0.0, "before backup")]).unwrap();
    let file = dir.path().join("manual.sqlite");
    ws.backup_to(&file).unwrap();
    assert!(ws.auto_backup().unwrap().is_some());
    assert!(ws.auto_backup().unwrap().is_none(), "at most one automatic backup a day");

    ws.create_cards(&[note(&root, 0.0, 0.0, "after backup")]).unwrap();
    ws.restore_backup(&file).unwrap();
    let cards = ws.open_board(&root).unwrap().cards;
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].data["text"], "before backup");
    let restored_backups = std::fs::read_dir(&paths.backups)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("pre-restore-"))
        .count();
    assert_eq!(restored_backups, 1);

    let junk = dir.path().join("junk.sqlite");
    std::fs::write(&junk, b"not a database").unwrap();
    assert!(ws.restore_backup(&junk).is_err());
    assert_eq!(ws.open_board(&root).unwrap().cards.len(), 1, "a bad backup changes nothing");
}

#[test]
fn settings_round_trip_and_sanitise() {
    let ws = Workspace::in_memory().unwrap();
    let mut s = ws.settings().unwrap();
    assert_eq!(s.theme, "system");
    s.theme = "neon".into();
    s.wheel_zooms = true;
    let saved = ws.save_settings(&s).unwrap();
    assert_eq!(saved.theme, "system");
    assert!(ws.settings().unwrap().wheel_zooms);
}

#[test]
fn benchmark_board_is_fast_to_open() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(dir.path().join("data"));
    let mut ws = Workspace::open(&paths).unwrap();
    let root = ws.root_board_id().unwrap();
    // Full card count, small images to keep the test quick.
    let spec = BenchmarkSpec { image_size: (64, 48), ..Default::default() };
    assert!(spec.total_cards() >= 1000);
    let images = benchmark::synthetic_images(&spec);
    let (patch, assets) = benchmark::generate(&mut ws, &root, spec, &images).unwrap();
    assert_eq!(assets.len(), 300);
    let board = patch.cards.iter().find(|c| c.kind == "board" && c.board_id == root).unwrap();
    let child = board.data["childBoardId"].as_str().unwrap();

    let t = Instant::now();
    let view = ws.open_board(child).unwrap();
    let elapsed = t.elapsed();
    assert!(view.cards.len() >= 1000, "{} cards", view.cards.len());
    assert_eq!(view.child_boards.len(), 40);
    eprintln!("opened a {}-card board in {elapsed:?}", view.cards.len());
    assert!(elapsed.as_millis() < 2000, "opening took {elapsed:?}");

    // The whole generation is one undo step.
    let undone = ws.undo().unwrap().unwrap();
    assert!(undone.removed_cards.len() >= 1000);
    assert!(ws.open_board(child).is_err());
}
