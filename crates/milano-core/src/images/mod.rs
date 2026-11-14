//! Image probing and thumbnail generation.

mod cache;
mod thumbnailer;

use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::webp::WebPEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};

pub use cache::ByteLru;
pub use thumbnailer::{Thumbnailer, Variant};

use crate::Result;

/// Longest-edge sizes generated for every image.
pub const THUMB_SIZES: [u32; 2] = [256, 1024];
const JPEG_QUALITY: u8 = 82;

fn swaps_axes(o: Orientation) -> bool {
    matches!(
        o,
        Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH
    )
}

/// Displayed dimensions (EXIF orientation applied) from the header only.
pub fn probe(path: &Path) -> Option<(u32, u32)> {
    let mut dec = ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_decoder().ok()?;
    let (w, h) = dec.dimensions();
    let o = dec.orientation().unwrap_or(Orientation::NoTransforms);
    Some(if swaps_axes(o) { (h, w) } else { (w, h) })
}

/// Decode fully, applying EXIF orientation.
pub fn decode(path: &Path) -> Result<DynamicImage> {
    let mut dec = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
    let o = dec.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(dec)?;
    img.apply_orientation(o);
    Ok(img)
}

pub fn thumbnail_candidates(dir: &Path, id: &str, size: u32) -> [PathBuf; 2] {
    let base = dir.join(&id[..2]);
    [base.join(format!("{id}-{size}.jpg")), base.join(format!("{id}-{size}.webp"))]
}

pub fn find_thumbnail(dir: &Path, id: &str, size: u32) -> Option<PathBuf> {
    thumbnail_candidates(dir, id, size).into_iter().find(|p| p.exists())
}

/// Encode a resized copy: JPEG for opaque images, lossless WebP when the
/// image has transparency (JPEG can't carry alpha).
pub fn encode_thumbnail(img: &DynamicImage, size: u32) -> Result<(Vec<u8>, &'static str)> {
    let small = if img.width().max(img.height()) > size { img.thumbnail(size, size) } else { img.clone() };
    let mut out = Vec::new();
    if small.color().has_alpha() {
        let rgba = small.to_rgba8();
        WebPEncoder::new_lossless(Cursor::new(&mut out)).encode(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        Ok((out, "webp"))
    } else {
        let rgb = small.to_rgb8();
        JpegEncoder::new_with_quality(BufWriter::new(Cursor::new(&mut out)), JPEG_QUALITY).encode_image(&rgb)?;
        Ok((out, "jpg"))
    }
}

/// Generate every thumbnail smaller than the source, decoding it once.
/// Returns the sizes written. Files are written atomically (temp + rename).
pub fn generate_thumbnails(src: &Path, thumbs_dir: &Path, id: &str) -> Result<Vec<u32>> {
    let img = decode(src)?;
    let longest = img.width().max(img.height());
    let dir = thumbs_dir.join(&id[..2]);
    std::fs::create_dir_all(&dir)?;
    let mut written = Vec::new();
    // Largest first, then downscale from it: cheaper than from the original.
    let mut source = img;
    for size in THUMB_SIZES.iter().rev().copied() {
        if size >= longest {
            continue;
        }
        let (bytes, ext) = encode_thumbnail(&source, size)?;
        let dest = dir.join(format!("{id}-{size}.{ext}"));
        let tmp = dir.join(format!(".{id}-{size}.{ext}.tmp"));
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &dest)?;
        written.push(size);
        source = source.thumbnail(size, size);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    #[test]
    fn writes_jpeg_and_webp_thumbnails() {
        let dir = tempfile::tempdir().unwrap();
        let id = "ab".repeat(32);
        let src = dir.path().join("big.png");
        RgbImage::from_pixel(2000, 1000, Rgb([10, 200, 30])).save(&src).unwrap();
        assert_eq!(probe(&src), Some((2000, 1000)));
        let sizes = generate_thumbnails(&src, dir.path(), &id).unwrap();
        assert_eq!(sizes, vec![1024, 256]);
        let t = find_thumbnail(dir.path(), &id, 256).unwrap();
        assert!(t.to_string_lossy().ends_with(".jpg"));
        assert_eq!(probe(&t), Some((256, 128)));

        let id2 = "cd".repeat(32);
        let alpha = dir.path().join("alpha.png");
        RgbaImage::from_pixel(600, 300, Rgba([0, 0, 0, 10])).save(&alpha).unwrap();
        assert_eq!(generate_thumbnails(&alpha, dir.path(), &id2).unwrap(), vec![256]);
        assert!(find_thumbnail(dir.path(), &id2, 256).unwrap().to_string_lossy().ends_with(".webp"));
        assert!(find_thumbnail(dir.path(), &id2, 1024).is_none());
    }
}
