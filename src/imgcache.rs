//! Keep a cache of generated thumbnails.

use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha224};

use crate::images::Thumbnail;

/// Variable name to use a specific cache for this program.
const CACHE_DIR_ENV: &str = "LIST_IMAGES_CACHE";

pub struct Cache {
    thumbnail_size: u32,

    cache_dir: PathBuf,
}

impl Cache {
    pub fn new(thumbnail_size: u32) -> Option<Cache> {
        let cache_dir = if let Some(value) = env::var_os(CACHE_DIR_ENV) {
            PathBuf::from(value)
        } else {
            let path = dirs::cache_dir()?;
            path.join(env!("CARGO_PKG_NAME"))
        };

        Some(Cache {
            thumbnail_size,
            cache_dir,
        })
    }

    pub fn get(&self, path: &Path) -> Option<Thumbnail> {
        let cached_path = self.file_hash(path)?;
        let data = std::fs::read(cached_path).ok()?;

        let header = turbojpeg::read_header(&data).ok()?;

        let thumbnail = Thumbnail {
            width: header.width as u32,
            height: header.height as u32,
            pixels: data,
        };

        Some(thumbnail)
    }

    pub fn store(&self, path: &Path, thumbnail: &Thumbnail) {
        if let Some(cached_path) = self.file_hash(path) {
            if let Some(parent) = cached_path.parent() {
                if std::fs::create_dir_all(parent).is_err() {
                    return;
                }
            }

            let _ = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(cached_path)
                .and_then(|mut file| file.write_all(&thumbnail.pixels));
        }
    }

    fn file_hash(&self, path: &Path) -> Option<PathBuf> {
        let mut hash = Sha224::new();

        hash.update(self.thumbnail_size.to_ne_bytes());

        if let Ok(metadata) = std::fs::metadata(path) {
            // Build a hash using data from the metadata.
            hash.update(metadata.len().to_ne_bytes());
            hash.update(metadata.mtime().to_ne_bytes());
            hash.update(metadata.dev().to_ne_bytes());
            hash.update(metadata.ino().to_ne_bytes());
        } else {
            // If metadata is not available, use the full path.
            hash.update(path.as_os_str().as_bytes());
        }

        let hash = hex::encode(hash.finalize());
        let (prefix, filename) = hash.split_at(2);
        Some(self.cache_dir.join(prefix).join(filename))
    }
}
