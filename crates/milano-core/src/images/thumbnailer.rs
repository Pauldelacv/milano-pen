//! Serves asset bytes for the webview and generates thumbnails on a bounded
//! worker pool. Each asset has its own lock, so a thumbnail requested by the
//! UI while the background prefetch is generating it waits instead of being
//! generated twice.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

use super::{find_thumbnail, generate_thumbnails, ByteLru, THUMB_SIZES};
use crate::files::{is_asset_id, mime_for_ext, AssetStore};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Original,
    Thumb(u32),
}

impl Variant {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "original" => Some(Self::Original),
            _ => s.parse::<u32>().ok().filter(|n| THUMB_SIZES.contains(n)).map(Self::Thumb),
        }
    }
}

pub struct Thumbnailer {
    store: AssetStore,
    workers: Arc<Semaphore>,
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    failed: Mutex<HashSet<String>>,
    cache: Mutex<ByteLru>,
}

fn mime_of(path: &std::path::Path) -> &'static str {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    mime_for_ext(&ext)
}

impl Thumbnailer {
    pub fn new(store: AssetStore, max_workers: usize, cache_bytes: usize) -> Self {
        Self {
            store,
            workers: Arc::new(Semaphore::new(max_workers.max(1))),
            locks: Mutex::new(HashMap::new()),
            failed: Mutex::new(HashSet::new()),
            cache: Mutex::new(ByteLru::new(cache_bytes)),
        }
    }

    /// A sensible pool size: half the cores, between 1 and 4.
    pub fn default_workers() -> usize {
        std::thread::available_parallelism().map(|n| n.get() / 2).unwrap_or(2).clamp(1, 4)
    }

    fn lock_for(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.locks.lock().expect("locks poisoned").entry(id.to_string()).or_default().clone()
    }

    /// Make sure every thumbnail of `id` exists (background prefetch).
    pub async fn ensure(&self, id: &str) -> Result<()> {
        if !is_asset_id(id) || self.failed.lock().expect("poisoned").contains(id) {
            return Ok(());
        }
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;
        let thumbs = self.store.thumbs_dir.clone();
        if THUMB_SIZES.iter().all(|s| find_thumbnail(&thumbs, id, *s).is_some()) {
            return Ok(());
        }
        let Some(src) = self.store.find_original(id) else {
            return Err(Error::NotFound(format!("asset {id}")));
        };
        if !mime_of(&src).starts_with("image/") {
            return Ok(());
        }
        // Already generated once (sizes ≥ the source are never written).
        let marker = thumbs.join(&id[..2]).join(format!("{id}.done"));
        if marker.exists() {
            return Ok(());
        }
        let _permit = self.workers.clone().acquire_owned().await.expect("semaphore closed");
        let id_owned = id.to_string();
        let result = tokio::task::spawn_blocking(move || {
            let r = generate_thumbnails(&src, &thumbs, &id_owned);
            if r.is_ok() {
                let _ = std::fs::write(&marker, b"");
            }
            r
        })
        .await
        .map_err(|e| Error::Invalid(format!("thumbnail worker failed: {e}")))?;
        if let Err(e) = result {
            log::warn!("no thumbnail for {id}: {e}");
            self.failed.lock().expect("poisoned").insert(id.to_string());
        }
        Ok(())
    }

    /// Bytes and MIME type for `id`/`variant`. A thumbnail that can't exist
    /// (source smaller than the size, or an undecodable format) falls back
    /// to the original.
    pub async fn load(&self, id: &str, variant: Variant) -> Result<(Arc<Vec<u8>>, &'static str)> {
        if !is_asset_id(id) {
            return Err(Error::Invalid("bad asset id".into()));
        }
        let path = match variant {
            Variant::Original => None,
            Variant::Thumb(size) => {
                let key = format!("{id}/{size}");
                if let Some(hit) = self.cache.lock().expect("poisoned").get(&key) {
                    let mime =
                        find_thumbnail(&self.store.thumbs_dir, id, size).map(|p| mime_of(&p)).unwrap_or("image/jpeg");
                    return Ok((hit, mime));
                }
                let mut found = find_thumbnail(&self.store.thumbs_dir, id, size);
                if found.is_none() {
                    self.ensure(id).await?;
                    found = find_thumbnail(&self.store.thumbs_dir, id, size);
                }
                if let Some(p) = &found {
                    let bytes = Arc::new(tokio::fs::read(p).await?);
                    self.cache.lock().expect("poisoned").insert(key, bytes.clone());
                    return Ok((bytes, mime_of(p)));
                }
                None::<PathBuf>
            }
        };
        let original =
            path.or_else(|| self.store.find_original(id)).ok_or_else(|| Error::NotFound(format!("asset {id}")))?;
        let bytes = Arc::new(tokio::fs::read(&original).await?);
        Ok((bytes, mime_of(&original)))
    }

    pub fn original_path(&self, id: &str) -> Option<PathBuf> {
        self.store.find_original(id)
    }

    pub fn forget(&self, id: &str) {
        self.cache.lock().expect("poisoned").remove_prefix(&format!("{id}/"));
        self.failed.lock().expect("poisoned").remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn generates_on_demand_and_falls_back_to_original() {
        let dir = tempfile::tempdir().unwrap();
        let store = AssetStore::new(dir.path().join("a"), dir.path().join("t"), dir.path().join("tmp"));
        let conn = crate::database::open_in_memory().unwrap();
        let src = dir.path().join("photo.png");
        RgbImage::from_pixel(1500, 500, Rgb([1, 2, 3])).save(&src).unwrap();
        let big = store.import_path(&conn, &src).unwrap();
        let small_src = dir.path().join("small.png");
        RgbImage::from_pixel(100, 100, Rgb([9, 9, 9])).save(&small_src).unwrap();
        let small = store.import_path(&conn, &small_src).unwrap();

        let t = Arc::new(Thumbnailer::new(store, 2, 1 << 20));
        // Two concurrent requests for the same asset: one generation.
        let (a, b) = tokio::join!(t.load(&big.id, Variant::Thumb(256)), t.load(&big.id, Variant::Thumb(256)));
        let (a, b) = (a.unwrap(), b.unwrap());
        assert_eq!(a.1, "image/jpeg");
        assert_eq!(a.0.len(), b.0.len());
        let (orig, mime) = t.load(&small.id, Variant::Thumb(1024)).await.unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(orig.len() as i64, small.size);
        assert!(t.load("nope", Variant::Original).await.is_err());
        assert_eq!(Variant::parse("256"), Some(Variant::Thumb(256)));
        assert_eq!(Variant::parse("300"), None);
    }
}
