//! IPC commands. Every command is `async` (so it never runs on the main
//! thread) and delegates to `milano-core` through [`AppState::with`].

pub mod assets;
pub mod boards;
pub mod cards;
pub mod files;
pub mod system;

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, FilePath};

/// Show a native "open" dialog and wait for the answer.
pub(crate) async fn pick_files<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    filters: &[(&str, &[&str])],
    multiple: bool,
) -> Vec<std::path::PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut d = app.dialog().file().set_title(title);
    for (name, exts) in filters {
        d = d.add_filter(*name, exts);
    }
    let cb = move |paths: Option<Vec<FilePath>>| {
        let _ = tx.send(paths.unwrap_or_default());
    };
    if multiple {
        d.pick_files(cb);
    } else {
        d.pick_file(move |p| cb(p.map(|p| vec![p])));
    }
    rx.await.unwrap_or_default().into_iter().filter_map(|p| p.into_path().ok()).collect()
}

/// Show a native "save" dialog and wait for the answer.
pub(crate) async fn pick_save<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    default_name: &str,
    filter: (&str, &[&str]),
) -> Option<std::path::PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().set_title(title).set_file_name(default_name).add_filter(filter.0, filter.1).save_file(
        move |p| {
            let _ = tx.send(p);
        },
    );
    rx.await.ok().flatten().and_then(|p| p.into_path().ok())
}

/// A file name safe on every file system.
pub(crate) fn safe_file_name(s: &str, fallback: &str) -> String {
    let cleaned: String =
        s.chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                    ' '
                } else {
                    c
                }
            })
            .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.chars().take(100).collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_names_are_safe() {
        assert_eq!(super::safe_file_name("a/b: c?", "x"), "a b c");
        assert_eq!(super::safe_file_name("  ", "board"), "board");
    }
}
