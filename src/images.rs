use image::{DynamicImage, RgbImage};
use std::ops::Deref;
use std::path::Path;
use turbojpeg::Subsamp;

/// Load an image from a file and returns the contents of a thumbnail.
pub fn thumbnail<P: AsRef<Path>>(path: &P, size: u32) -> anyhow::Result<impl Deref<Target = [u8]>> {
    let image = load_file(path)?;

    let thumbnail = DynamicImage::ImageRgb8(image).thumbnail(size, size);
    Ok(turbojpeg::compress_image(
        thumbnail.as_rgb8().expect("thumbnail must be ImageRgb8"),
        90,
        Subsamp::None,
    )?)
}

fn load_file<P: AsRef<Path>>(path: &P) -> anyhow::Result<RgbImage> {
    let data = std::fs::read(path.as_ref())?;

    // If this file is identified as a JPEG, try to load it with turbojpeg. If
    // it fails, fallback to JPEG decoder in the image crate.
    if data.get(0..2) == Some(&[0xFF, 0xD8]) {
        if let Ok(img) = turbojpeg::decompress_image(&data) {
            return Ok(img);
        }
    }

    Ok(image::load_from_memory(&data)?.into_rgb8())
}
