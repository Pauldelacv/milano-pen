//! Content-addressed asset store.
//!
//! Originals are copied (never moved or modified) to
//! `assets/<first two hex chars>/<sha256>.<ext>`. The same content imported
//! twice is stored once.

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::images;
use crate::models::Asset;
use crate::{new_id, now_ms, Error, Result};

/// Imports larger than this are refused.
pub const MAX_ASSET_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct AssetStore {
    pub assets_dir: PathBuf,
    pub thumbs_dir: PathBuf,
    tmp_dir: PathBuf,
}

/// True for a well-formed asset id (lower-case sha256 hex).
pub fn is_asset_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A safe, lower-case extension (alphanumeric, ≤ 10 chars) or "bin".
pub fn clean_ext(name: &str) -> String {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !ext.is_empty() && ext.len() <= 10 && ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        ext
    } else {
        "bin".into()
    }
}

/// MIME type from the extension (the app never executes or renders untrusted
/// formats based on it; it only picks a card kind and a preview).
pub fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "ico" => "image/x-icon",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "zip" => "application/zip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "key" => "application/x-iwork-keynote-sffkey",
        "pages" => "application/x-iwork-pages-sffpages",
        "numbers" => "application/x-iwork-numbers-sffnumbers",
        _ => "application/octet-stream",
    }
}

impl AssetStore {
    pub fn new(assets_dir: impl Into<PathBuf>, thumbs_dir: impl Into<PathBuf>, tmp_dir: impl Into<PathBuf>) -> Self {
        Self { assets_dir: assets_dir.into(), thumbs_dir: thumbs_dir.into(), tmp_dir: tmp_dir.into() }
    }

    pub fn original_path(&self, id: &str, ext: &str) -> PathBuf {
        self.assets_dir.join(&id[..2]).join(format!("{id}.{ext}"))
    }

    /// Find an original by id without a database lookup.
    pub fn find_original(&self, id: &str) -> Option<PathBuf> {
        if !is_asset_id(id) {
            return None;
        }
        let dir = self.assets_dir.join(&id[..2]);
        let prefix = format!("{id}.");
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix)))
    }

    /// Copy a file into the store and register it. Reads it once (hashing
    /// while copying) and only the image header afterwards.
    pub fn import_path(&self, conn: &Connection, src: &Path) -> Result<Asset> {
        let meta = std::fs::metadata(src)?;
        if !meta.is_file() {
            return Err(Error::Invalid(format!("{} is not a file", src.display())));
        }
        if meta.len() > MAX_ASSET_BYTES {
            return Err(Error::Invalid("files over 2 GB can't be imported".into()));
        }
        let name = src.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
        let reader = BufReader::new(File::open(src)?);
        self.import_reader(conn, &name, reader)
    }

    pub fn import_bytes(&self, conn: &Connection, name: &str, bytes: &[u8]) -> Result<Asset> {
        if bytes.len() as u64 > MAX_ASSET_BYTES {
            return Err(Error::Invalid("files over 2 GB can't be imported".into()));
        }
        self.import_reader(conn, name, bytes)
    }

    fn import_reader(&self, conn: &Connection, name: &str, mut reader: impl Read) -> Result<Asset> {
        std::fs::create_dir_all(&self.tmp_dir)?;
        let tmp = self.tmp_dir.join(format!("import-{}", new_id()));
        let mut out = File::create(&tmp)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 256 * 1024];
        let mut size: u64 = 0;
        let copied = (|| -> Result<()> {
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                size += n as u64;
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])?;
            }
            out.sync_all()?;
            Ok(())
        })();
        drop(out);
        if let Err(e) = copied {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        let id = format!("{:x}", hasher.finalize());
        let ext = clean_ext(name);
        let dest = self.original_path(&id, &ext);
        if dest.exists() {
            std::fs::remove_file(&tmp)?;
        } else {
            std::fs::create_dir_all(dest.parent().expect("asset path has a parent"))?;
            std::fs::rename(&tmp, &dest)?;
        }
        if let Some(existing) = self.get(conn, &id)? {
            return Ok(existing);
        }
        let mime = mime_for_ext(&ext).to_string();
        let dims = if mime.starts_with("image/") { images::probe(&dest) } else { None };
        let asset = Asset {
            id,
            ext,
            mime,
            size: size as i64,
            width: dims.map(|d| d.0 as i64),
            height: dims.map(|d| d.1 as i64),
            original_name: name.chars().take(255).collect(),
            created_at: now_ms(),
        };
        conn.execute(
            "INSERT OR IGNORE INTO assets (id, ext, mime, size, width, height, original_name, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                asset.id,
                asset.ext,
                asset.mime,
                asset.size,
                asset.width,
                asset.height,
                asset.original_name,
                asset.created_at
            ],
        )?;
        Ok(asset)
    }

    /// Register an asset copied from another store (same id, same bytes).
    /// Returns false if this store already had it.
    pub fn adopt(&self, conn: &Connection, asset: &Asset, src: &Path) -> Result<bool> {
        if !is_asset_id(&asset.id) || clean_ext(&format!("x.{}", asset.ext)) != asset.ext {
            return Err(Error::Invalid("not a valid asset".into()));
        }
        if self.get(conn, &asset.id)?.is_some() {
            return Ok(false);
        }
        let dest = self.original_path(&asset.id, &asset.ext);
        if !dest.exists() {
            std::fs::create_dir_all(&self.tmp_dir)?;
            std::fs::create_dir_all(dest.parent().expect("asset path has a parent"))?;
            // Copy to a temporary file first so a crash never leaves a partial original.
            let tmp = self.tmp_dir.join(format!("adopt-{}", new_id()));
            if let Err(e) = std::fs::copy(src, &tmp).and_then(|_| std::fs::rename(&tmp, &dest)) {
                let _ = std::fs::remove_file(&tmp);
                return Err(e.into());
            }
        }
        conn.execute(
            "INSERT OR IGNORE INTO assets (id, ext, mime, size, width, height, original_name, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                asset.id,
                asset.ext,
                asset.mime,
                asset.size,
                asset.width,
                asset.height,
                asset.original_name,
                now_ms()
            ],
        )?;
        Ok(true)
    }

    pub fn get(&self, conn: &Connection, id: &str) -> Result<Option<Asset>> {
        Ok(conn
            .query_row(
                "SELECT id, ext, mime, size, width, height, original_name, created_at FROM assets WHERE id = ?1",
                [id],
                |r| {
                    Ok(Asset {
                        id: r.get(0)?,
                        ext: r.get(1)?,
                        mime: r.get(2)?,
                        size: r.get(3)?,
                        width: r.get(4)?,
                        height: r.get(5)?,
                        original_name: r.get(6)?,
                        created_at: r.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    /// Delete an asset's original and thumbnails. Returns true if the original existed.
    pub fn remove(&self, id: &str, ext: &str) -> bool {
        if !is_asset_id(id) {
            return false;
        }
        for size in images::THUMB_SIZES {
            for p in images::thumbnail_candidates(&self.thumbs_dir, id, size) {
                let _ = std::fs::remove_file(p);
            }
        }
        std::fs::remove_file(self.original_path(id, ext)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_sanitised() {
        assert_eq!(clean_ext("Photo.JPG"), "jpg");
        assert_eq!(clean_ext("archive.tar.gz"), "gz");
        assert_eq!(clean_ext("noext"), "bin");
        assert_eq!(clean_ext("evil.ph p"), "bin");
        assert_eq!(clean_ext("x.verylongextension"), "bin");
    }

    #[test]
    fn asset_ids_are_strict() {
        assert!(is_asset_id(&"a".repeat(64)));
        assert!(!is_asset_id(&"A".repeat(64)));
        assert!(!is_asset_id("../etc/passwd"));
    }
}
