//! FFI declarations for macOS getattrlistbulk system call.
//!
//! Reference: /usr/include/sys/attr.h
//!
//! # Safety
//!
//! All FFI functions in this module are unsafe. The safe wrappers
//! are provided in the parent module.

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use bitflags::bitflags;

/// Attribute list structure for getattrlistbulk
#[repr(C)]
pub struct attrlist {
    pub bitmapcount: u16,
    pub reserved: u16,
    pub commonattr: u32,
    pub volattr: u32,
    pub dirattr: u32,
    pub fileattr: u32,
    pub forkattr: u32,
}

/// Returned attribute set - indicates which attributes were actually returned
#[repr(C)]
pub struct attribute_set {
    pub commonattr: u32,
    pub volattr: u32,
    pub dirattr: u32,
    pub fileattr: u32,
    pub forkattr: u32,
}

/// Reference to variable-length attribute data
#[repr(C)]
pub struct attrreference {
    pub attr_dataoffset: i32,
    pub attr_length: u32,
}

pub const ATTR_BIT_MAP_COUNT: u16 = 5;

bitflags! {
    /// Common attributes (commonattr field)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CommonAttr: u32 {
        const RETURNED_ATTRS = 0x80000000;
        const NAME = 0x00000001;
        const OBJTYPE = 0x00000008;
        const MODTIME = 0x00000400;
        const ACCESSMASK = 0x00020000;
        const FILEID = 0x02000000;
    }

    /// File-specific attributes (fileattr field)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileAttr: u32 {
        const TOTALSIZE = 0x00000002;
        const ALLOCSIZE = 0x00000004;
        const DATALENGTH = 0x00000200;
    }

    /// Directory-specific attributes (dirattr field)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DirAttr: u32 {
        const ENTRYCOUNT = 0x00000002;
    }

    /// Options for getattrlistbulk
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FsOptions: u64 {
        const NOFOLLOW = 0x00000001;
        const PACK_INVAL_ATTRS = 0x00000008;
    }
}

extern "C" {
    /// Bulk directory enumeration with attribute retrieval.
    ///
    /// # Safety
    ///
    /// - `dirfd` must be a valid open directory file descriptor
    /// - `alist` must point to a valid attrlist structure
    /// - `attribute_buffer` must point to a buffer of at least `buffer_size` bytes
    pub fn getattrlistbulk(
        dirfd: libc::c_int,
        alist: *mut attrlist,
        attribute_buffer: *mut libc::c_void,
        buffer_size: libc::size_t,
        options: u64,
    ) -> libc::ssize_t;
}
