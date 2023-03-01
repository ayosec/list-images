//! Support for archives using `libarchive`.

use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use libarchive3_sys::ffi;

pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<impl Iterator<Item = Entry>> {
    let entries = unsafe {
        let archive = ffi::archive_read_new();
        if archive.is_null() {
            anyhow::bail!("archive_read_new failed");
        }

        ffi::archive_read_support_filter_all(archive);
        ffi::archive_read_support_format_all(archive);

        ArchiveEntries { archive }
    };

    let res = unsafe {
        let cstr = CString::new(path.as_ref().as_os_str().as_bytes())?;
        ffi::archive_read_open_filename(entries.archive, cstr.as_ptr(), 16 * 1024)
    };

    if res != ffi::ARCHIVE_OK {
        anyhow::bail!("File is not an archive");
    }

    Ok(entries)
}

pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
}

struct ArchiveEntries {
    archive: *mut ffi::Struct_archive,
}

impl Iterator for ArchiveEntries {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut entry = std::ptr::null_mut();

            let res = unsafe { ffi::archive_read_next_header(self.archive, &mut entry) };
            if res != ffi::ARCHIVE_OK {
                return None;
            }

            // Skip non-regular files.
            let res = unsafe { ffi::archive_entry_filetype(entry) };
            if res != ffi::AE_IFREG {
                continue;
            }

            // Get size of the entry.
            let file_size = usize::try_from(unsafe { ffi::archive_entry_size(entry) }).unwrap_or(0);
            if file_size == 0 {
                continue;
            }

            // Get name of the entry.
            let c_name = unsafe { CStr::from_ptr(ffi::archive_entry_pathname(entry)).to_bytes() };
            let name = String::from_utf8_lossy(c_name).into_owned();

            // Get contents of the entry.
            let mut data: Vec<u8> = vec![0; file_size];
            let res = unsafe {
                ffi::archive_read_data(self.archive, data.as_mut_ptr().cast(), data.len())
            };

            if res as usize != data.len() {
                return None;
            }

            return Some(Entry { name, data });
        }
    }
}

impl Drop for ArchiveEntries {
    fn drop(&mut self) {
        unsafe { ffi::archive_read_free(self.archive) };
        self.archive = std::ptr::null_mut();
    }
}
