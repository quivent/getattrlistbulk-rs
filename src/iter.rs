//! Iterator implementation for directory entries.
//!
//! This module provides the `DirEntries` iterator that lazily reads
//! directory entries using getattrlistbulk.

use crate::error::Error;
use crate::ffi;
use crate::parser::BufferParser;
use crate::types::{DirEntry, RequestedAttributes};
use std::os::unix::io::RawFd;
use std::path::Path;

/// Iterator over directory entries.
///
/// Yields `Result<DirEntry, Error>` for each entry in the directory.
/// Automatically refills the internal buffer as needed.
///
/// # Thread Safety
///
/// `DirEntries` implements `Send` but not `Sync`. This means:
/// - You can move a `DirEntries` to another thread
/// - You cannot share a `DirEntries` between threads simultaneously
///
/// This is because `DirEntries` owns a file descriptor that must be
/// accessed exclusively.
///
/// # Example
///
/// ```no_run
/// use getattrlistbulk::{read_dir, RequestedAttributes};
///
/// let entries = read_dir("/tmp", RequestedAttributes::all())?;
/// for entry in entries {
///     let entry = entry?;
///     println!("{}", entry.name);
/// }
/// # Ok::<(), getattrlistbulk::Error>(())
/// ```
pub struct DirEntries {
    dirfd: RawFd,
    buffer: Vec<u8>,
    bytes_valid: usize,
    parser_offset: usize,
    requested: RequestedAttributes,
    exhausted: bool,
    follow_symlinks: bool,
}

// DirEntries owns the fd exclusively, safe to send between threads
unsafe impl Send for DirEntries {}

impl DirEntries {
    /// Create a new directory iterator.
    pub(crate) fn new(
        path: &Path,
        requested: RequestedAttributes,
        buffer_size: usize,
        follow_symlinks: bool,
    ) -> Result<Self, Error> {
        let dirfd = open_directory(path)?;

        Ok(Self {
            dirfd,
            buffer: vec![0u8; buffer_size],
            bytes_valid: 0,
            parser_offset: 0,
            requested,
            exhausted: false,
            follow_symlinks,
        })
    }

    /// Refill the buffer with more entries.
    ///
    /// Returns Ok(true) if entries were read, Ok(false) if exhausted.
    fn refill_buffer(&mut self) -> Result<bool, Error> {
        let mut attrlist: ffi::attrlist = self.requested.into();

        let mut options = ffi::FsOptions::PACK_INVAL_ATTRS;
        if !self.follow_symlinks {
            options |= ffi::FsOptions::NOFOLLOW;
        }

        let result = unsafe {
            ffi::getattrlistbulk(
                self.dirfd,
                &mut attrlist,
                self.buffer.as_mut_ptr() as *mut libc::c_void,
                self.buffer.len(),
                options.bits(),
            )
        };

        if result < 0 {
            let err = std::io::Error::last_os_error();
            // Handle EINTR by retrying
            if err.raw_os_error() == Some(libc::EINTR) {
                return self.refill_buffer();
            }
            return Err(Error::Syscall(err));
        }

        if result == 0 {
            self.exhausted = true;
            return Ok(false);
        }

        // result contains the number of entries, but we need bytes
        // The buffer format tells us how many bytes via entry lengths
        // For now, we'll track bytes as we parse
        self.bytes_valid = self.find_valid_bytes();
        self.parser_offset = 0;

        Ok(true)
    }

    /// Find how many bytes in the buffer contain valid entries.
    fn find_valid_bytes(&self) -> usize {
        let mut offset = 0;
        while offset + 4 <= self.buffer.len() {
            let entry_length = u32::from_ne_bytes(
                self.buffer[offset..offset + 4]
                    .try_into()
                    .unwrap_or([0; 4]),
            ) as usize;

            if entry_length == 0 {
                break;
            }

            if offset + entry_length > self.buffer.len() {
                break;
            }

            offset += entry_length;
        }
        offset
    }

    /// Parse the next entry from the current buffer position.
    fn next_from_buffer(&mut self) -> Option<Result<DirEntry, Error>> {
        if self.parser_offset >= self.bytes_valid {
            return None;
        }

        let mut parser = BufferParser::new(
            &self.buffer[self.parser_offset..],
            self.bytes_valid - self.parser_offset,
            self.requested,
        );

        match parser.next_entry() {
            Some(Ok(entry)) => {
                // Advance our offset by the entry length
                let entry_length = u32::from_ne_bytes(
                    self.buffer[self.parser_offset..self.parser_offset + 4]
                        .try_into()
                        .unwrap_or([0; 4]),
                ) as usize;
                self.parser_offset += entry_length;
                Some(Ok(entry))
            }
            Some(Err(e)) => Some(Err(Error::from(e))),
            None => None,
        }
    }
}

impl Iterator for DirEntries {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next entry from current buffer
            if let Some(result) = self.next_from_buffer() {
                return Some(result);
            }

            // Buffer exhausted, try to refill
            if self.exhausted {
                return None;
            }

            match self.refill_buffer() {
                Ok(true) => continue,
                Ok(false) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

impl Drop for DirEntries {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.dirfd);
        }
    }
}

/// Open a directory and return its file descriptor.
fn open_directory(path: &Path) -> Result<RawFd, Error> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| Error::Open(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "path contains null byte",
        )))?;

    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY) };

    if fd < 0 {
        return Err(Error::Open(std::io::Error::last_os_error()));
    }

    Ok(fd)
}
