use image::{DynamicImage, RgbImage};
use std::path::{Path, PathBuf};
use turbojpeg::Subsamp;

/// Maximum size for image files (32M).
const DEFAULT_MAX_IMAGE_FILE_SIZE: u64 = 32 << 20;

pub struct Thumbnail {
    pub height: u32,
    pub width: u32,
    pub pixels: Vec<u8>,
}

pub enum Source<'a> {
    Path(PathBuf),
    Mem(&'a [u8], PathBuf),
}

impl Source<'_> {
    pub fn path(&self) -> &Path {
        match self {
            Source::Path(path) => path,
            Source::Mem(_, path) => path,
        }
    }

    pub fn into_path_buf(self) -> PathBuf {
        match self {
            Source::Path(path) => path,
            Source::Mem(_, path) => path,
        }
    }
}

/// Load an image from a file and returns the contents of a thumbnail.
pub fn thumbnail(
    source: &Source,
    height: u32,
    width: u32,
    max_size: Option<u64>,
) -> anyhow::Result<Thumbnail> {
    let image = match source {
        Source::Mem(mem, _) => image::load_from_memory(mem)?.into_rgb8(),

        Source::Path(ref path) => {
            match load_file(path, max_size) {
                Ok(i) => i,
                Err(e) => {
                    // If the file can't be parsed as an image, try to capture a frame
                    // with ffmpeg.
                    if let Ok(frame) = crate::ffmpeg::get_frame(path.as_ref()) {
                        image::load_from_memory(&frame)?.into_rgb8()
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    };

    let thumbnail = DynamicImage::ImageRgb8(image).thumbnail(height, width);
    let buf = turbojpeg::compress_image(
        thumbnail.as_rgb8().expect("thumbnail must be ImageRgb8"),
        90,
        Subsamp::None,
    )?;

    let pixels = buf.as_ref().into();

    Ok(Thumbnail {
        height: thumbnail.height(),
        width: thumbnail.width(),
        pixels,
    })
}

fn load_file<P: AsRef<Path>>(path: &P, max_size: Option<u64>) -> anyhow::Result<RgbImage> {
    let metadata = std::fs::metadata(path.as_ref())?;

    let max_size = max_size.unwrap_or(DEFAULT_MAX_IMAGE_FILE_SIZE);

    if metadata.len() > max_size {
        anyhow::bail!(
            "File exceeds the maximum size ({} > {})",
            metadata.len(),
            max_size
        );
    }

    if !metadata.is_file() {
        anyhow::bail!("Expected a regular file");
    }

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
