//! # getattrlistbulk
//!
//! Safe Rust bindings for the macOS `getattrlistbulk()` system call.
//!
//! This crate provides high-performance directory enumeration by retrieving
//! file metadata in bulk, reducing the number of syscalls from O(n) to O(n/batch_size).
//!
//! ## Example
//!
//! ```no_run
//! use getattrlistbulk::{read_dir, RequestedAttributes};
//!
//! let attrs = RequestedAttributes {
//!     name: true,
//!     size: true,
//!     ..Default::default()
//! };
//!
//! for entry in read_dir("/Users", attrs).unwrap() {
//!     let entry = entry.unwrap();
//!     println!("{}: {} bytes", entry.name, entry.size.unwrap_or(0));
//! }
//! ```
//!
//! ## Platform Support
//!
//! This crate only compiles on macOS. Attempting to compile on other platforms
//! will result in a compile-time error.

#![cfg(target_os = "macos")]

#[cfg(not(target_os = "macos"))]
compile_error!("getattrlistbulk is only available on macOS");

mod ffi;
mod types;
mod parser;
mod iter;
mod error;
mod builder;

pub use types::{RequestedAttributes, ObjectType, DirEntry};
pub use error::Error;
pub use iter::DirEntries;
pub use builder::DirReader;

use std::path::Path;

/// Read directory entries with specified attributes.
///
/// Uses a default buffer size of 64KB. For large directories,
/// consider using [`read_dir_with_buffer`] with a larger buffer.
///
/// # Example
///
/// ```no_run
/// use getattrlistbulk::{read_dir, RequestedAttributes};
///
/// let attrs = RequestedAttributes {
///     name: true,
///     size: true,
///     object_type: true,
///     ..Default::default()
/// };
///
/// for entry in read_dir("/tmp", attrs)? {
///     println!("{:?}", entry?);
/// }
/// # Ok::<(), getattrlistbulk::Error>(())
/// ```
pub fn read_dir<P: AsRef<Path>>(
    path: P,
    attrs: RequestedAttributes,
) -> Result<DirEntries, Error> {
    read_dir_with_buffer(path, attrs, 64 * 1024)
}

/// Read directory entries with custom buffer size.
///
/// Larger buffers result in fewer syscalls but use more memory.
/// Recommended buffer sizes:
/// - 64KB: Default, good for most directories
/// - 256KB: Large directories (10,000+ files)
/// - 1MB: Very large directories (100,000+ files)
pub fn read_dir_with_buffer<P: AsRef<Path>>(
    path: P,
    attrs: RequestedAttributes,
    buffer_size: usize,
) -> Result<DirEntries, Error> {
    DirEntries::new(path.as_ref(), attrs, buffer_size, true)
}
