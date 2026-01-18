//! Rust types and conversions for getattrlistbulk.
//!
//! This module defines the public types used by the crate and implements
//! conversions between Rust types and FFI types.

use crate::ffi;
use std::time::SystemTime;

// TODO: Task B - Implement types and conversions
// See IMPLEMENTATION.md Task B for requirements

/// Attributes to request for each directory entry.
///
/// Set fields to `true` to request those attributes. Only requested
/// attributes will be retrieved, which can improve performance.
///
/// # Example
///
/// ```
/// use getattrlistbulk::RequestedAttributes;
///
/// // Request only name and size
/// let attrs = RequestedAttributes {
///     name: true,
///     size: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestedAttributes {
    /// File or directory name
    pub name: bool,
    /// Object type (file, directory, symlink, etc.)
    pub object_type: bool,
    /// Total size in bytes
    pub size: bool,
    /// Allocated size on disk
    pub alloc_size: bool,
    /// Last modification time
    pub modified_time: bool,
    /// Unix permissions mask
    pub permissions: bool,
    /// Inode number / file ID
    pub inode: bool,
    /// Entry count (directories only)
    pub entry_count: bool,
}

impl RequestedAttributes {
    /// Request all available attributes.
    pub fn all() -> Self {
        Self {
            name: true,
            object_type: true,
            size: true,
            alloc_size: true,
            modified_time: true,
            permissions: true,
            inode: true,
            entry_count: true,
        }
    }

    /// Builder method to request name.
    pub fn with_name(mut self) -> Self {
        self.name = true;
        self
    }

    /// Builder method to request object type.
    pub fn with_object_type(mut self) -> Self {
        self.object_type = true;
        self
    }

    /// Builder method to request size.
    pub fn with_size(mut self) -> Self {
        self.size = true;
        self
    }

    /// Builder method to request allocation size.
    pub fn with_alloc_size(mut self) -> Self {
        self.alloc_size = true;
        self
    }

    /// Builder method to request modification time.
    pub fn with_modified_time(mut self) -> Self {
        self.modified_time = true;
        self
    }

    /// Builder method to request permissions.
    pub fn with_permissions(mut self) -> Self {
        self.permissions = true;
        self
    }

    /// Builder method to request inode.
    pub fn with_inode(mut self) -> Self {
        self.inode = true;
        self
    }

    /// Builder method to request entry count.
    pub fn with_entry_count(mut self) -> Self {
        self.entry_count = true;
        self
    }
}

impl From<RequestedAttributes> for ffi::attrlist {
    fn from(req: RequestedAttributes) -> Self {
        let mut common = ffi::CommonAttr::RETURNED_ATTRS;
        let mut file = ffi::FileAttr::empty();
        let mut dir = ffi::DirAttr::empty();

        if req.name {
            common |= ffi::CommonAttr::NAME;
        }
        if req.object_type {
            common |= ffi::CommonAttr::OBJTYPE;
        }
        if req.modified_time {
            common |= ffi::CommonAttr::MODTIME;
        }
        if req.permissions {
            common |= ffi::CommonAttr::ACCESSMASK;
        }
        if req.inode {
            common |= ffi::CommonAttr::FILEID;
        }
        if req.size {
            file |= ffi::FileAttr::TOTALSIZE;
        }
        if req.alloc_size {
            file |= ffi::FileAttr::ALLOCSIZE;
        }
        if req.entry_count {
            dir |= ffi::DirAttr::ENTRYCOUNT;
        }

        ffi::attrlist {
            bitmapcount: ffi::ATTR_BIT_MAP_COUNT,
            reserved: 0,
            commonattr: common.bits(),
            volattr: 0,
            dirattr: dir.bits(),
            fileattr: file.bits(),
            forkattr: 0,
        }
    }
}

/// Type of filesystem object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
    /// Block device
    BlockDevice,
    /// Character device
    CharDevice,
    /// Socket
    Socket,
    /// Named pipe (FIFO)
    Fifo,
    /// Unknown type
    Unknown(u32),
}

impl From<u32> for ObjectType {
    fn from(vtype: u32) -> Self {
        // Values from sys/vnode.h
        match vtype {
            1 => ObjectType::Regular,   // VREG
            2 => ObjectType::Directory, // VDIR
            5 => ObjectType::Symlink,   // VLNK
            3 => ObjectType::BlockDevice, // VBLK
            4 => ObjectType::CharDevice,  // VCHR
            6 => ObjectType::Socket,    // VSOCK
            7 => ObjectType::Fifo,      // VFIFO
            v => ObjectType::Unknown(v),
        }
    }
}

/// Metadata for a single directory entry.
///
/// # String Handling
///
/// The `name` field uses lossy UTF-8 conversion. If a filename contains
/// invalid UTF-8 sequences (rare on macOS but possible), invalid bytes
/// are replaced with the Unicode replacement character (U+FFFD).
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// File or directory name
    pub name: String,
    /// Object type
    pub object_type: Option<ObjectType>,
    /// Total size in bytes
    pub size: Option<u64>,
    /// Allocated size on disk
    pub alloc_size: Option<u64>,
    /// Last modification time
    pub modified_time: Option<SystemTime>,
    /// Unix permissions mask
    pub permissions: Option<u32>,
    /// Inode number / file ID
    pub inode: Option<u64>,
    /// Entry count (directories only)
    pub entry_count: Option<u32>,
}

impl DirEntry {
    /// Check if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.object_type == Some(ObjectType::Directory)
    }

    /// Check if this entry is a regular file.
    pub fn is_file(&self) -> bool {
        self.object_type == Some(ObjectType::Regular)
    }

    /// Check if this entry is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.object_type == Some(ObjectType::Symlink)
    }
}
