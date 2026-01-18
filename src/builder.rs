//! Builder pattern API for directory reading.
//!
//! Provides a fluent interface for configuring directory reads.

use crate::error::Error;
use crate::iter::DirEntries;
use crate::types::RequestedAttributes;
use std::path::{Path, PathBuf};

/// Builder for configuring directory reads.
///
/// # Example
///
/// ```no_run
/// use getattrlistbulk::DirReader;
///
/// let entries = DirReader::new("/tmp")
///     .name()
///     .size()
///     .object_type()
///     .buffer_size(128 * 1024)
///     .read()?;
///
/// for entry in entries {
///     let entry = entry?;
///     println!("{}: {:?} bytes", entry.name, entry.size);
/// }
/// # Ok::<(), getattrlistbulk::Error>(())
/// ```
pub struct DirReader {
    path: PathBuf,
    attrs: RequestedAttributes,
    buffer_size: usize,
    follow_symlinks: bool,
}

impl DirReader {
    /// Create a new directory reader for the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            attrs: RequestedAttributes::default(),
            buffer_size: 64 * 1024,
            follow_symlinks: true,
        }
    }

    /// Request file/directory names.
    pub fn name(mut self) -> Self {
        self.attrs.name = true;
        self
    }

    /// Request object types (file, directory, symlink, etc.).
    pub fn object_type(mut self) -> Self {
        self.attrs.object_type = true;
        self
    }

    /// Request file sizes.
    pub fn size(mut self) -> Self {
        self.attrs.size = true;
        self
    }

    /// Request allocated sizes on disk.
    pub fn alloc_size(mut self) -> Self {
        self.attrs.alloc_size = true;
        self
    }

    /// Request modification times.
    pub fn modified_time(mut self) -> Self {
        self.attrs.modified_time = true;
        self
    }

    /// Request Unix permissions.
    pub fn permissions(mut self) -> Self {
        self.attrs.permissions = true;
        self
    }

    /// Request inode numbers.
    pub fn inode(mut self) -> Self {
        self.attrs.inode = true;
        self
    }

    /// Request entry counts (for directories).
    pub fn entry_count(mut self) -> Self {
        self.attrs.entry_count = true;
        self
    }

    /// Request all available attributes.
    pub fn all_attributes(mut self) -> Self {
        self.attrs = RequestedAttributes::all();
        self
    }

    /// Set a custom buffer size.
    ///
    /// Larger buffers result in fewer syscalls but use more memory.
    /// Default is 64KB.
    ///
    /// Recommended values:
    /// - 64KB: Default, good for most directories
    /// - 256KB: Large directories (10,000+ files)
    /// - 1MB: Very large directories (100,000+ files)
    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set custom attributes to request.
    pub fn attributes(mut self, attrs: RequestedAttributes) -> Self {
        self.attrs = attrs;
        self
    }

    /// Control whether symbolic links are followed.
    ///
    /// When `false`, uses FSOPT_NOFOLLOW to not follow symlinks.
    /// Default is `true` (follow symlinks).
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Read the directory and return an iterator over entries.
    ///
    /// # Note
    ///
    /// The `name` attribute is always requested regardless of configuration,
    /// as entries without names are not useful. If you explicitly set
    /// `name: false`, it will be overridden to `true`.
    pub fn read(self) -> Result<DirEntries, Error> {
        // Ensure at least name is requested
        let mut attrs = self.attrs;
        if !attrs.name {
            attrs.name = true;
        }

        DirEntries::new(&self.path, attrs, self.buffer_size, self.follow_symlinks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_chaining() {
        let reader = DirReader::new("/tmp")
            .name()
            .size()
            .object_type()
            .buffer_size(128 * 1024);

        assert!(reader.attrs.name);
        assert!(reader.attrs.size);
        assert!(reader.attrs.object_type);
        assert!(!reader.attrs.permissions);
        assert_eq!(reader.buffer_size, 128 * 1024);
    }

    #[test]
    fn test_all_attributes() {
        let reader = DirReader::new("/tmp").all_attributes();

        assert!(reader.attrs.name);
        assert!(reader.attrs.size);
        assert!(reader.attrs.object_type);
        assert!(reader.attrs.permissions);
        assert!(reader.attrs.inode);
    }
}
