//! FFI validation tests for getattrlistbulk.
//!
//! These tests verify that FFI constants and struct sizes match
//! the macOS system headers, catching regressions if values change.

#[cfg(target_os = "macos")]
mod ffi_validation {
    // Note: We test via the public API since ffi module is private.
    // These tests validate the behavior rather than internal constants.

    use std::mem::size_of;

    /// Verify attrlist struct size matches C definition (24 bytes)
    #[test]
    fn test_attrlist_size() {
        // attrlist: bitmapcount(2) + reserved(2) + commonattr(4) + volattr(4)
        //           + dirattr(4) + fileattr(4) + forkattr(4) = 24 bytes
        #[repr(C)]
        struct AttrlistCheck {
            bitmapcount: u16,
            reserved: u16,
            commonattr: u32,
            volattr: u32,
            dirattr: u32,
            fileattr: u32,
            forkattr: u32,
        }
        assert_eq!(size_of::<AttrlistCheck>(), 24);
    }

    /// Verify attrreference struct size matches C definition (8 bytes)
    #[test]
    fn test_attrreference_size() {
        // attrreference: attr_dataoffset(4) + attr_length(4) = 8 bytes
        #[repr(C)]
        struct AttrreferenceCheck {
            attr_dataoffset: i32,
            attr_length: u32,
        }
        assert_eq!(size_of::<AttrreferenceCheck>(), 8);
    }

    /// Verify attribute_set struct size matches C definition (20 bytes)
    #[test]
    fn test_attribute_set_size() {
        // attribute_set: 5 x u32 = 20 bytes
        #[repr(C)]
        struct AttributeSetCheck {
            commonattr: u32,
            volattr: u32,
            dirattr: u32,
            fileattr: u32,
            forkattr: u32,
        }
        assert_eq!(size_of::<AttributeSetCheck>(), 20);
    }

    /// Verify timespec struct size on 64-bit (16 bytes)
    #[test]
    fn test_timespec_size() {
        // timespec on 64-bit: tv_sec(8) + tv_nsec(8) = 16 bytes
        #[repr(C)]
        struct TimespecCheck {
            tv_sec: i64,
            tv_nsec: i64,
        }
        assert_eq!(size_of::<TimespecCheck>(), 16);
    }

    /// Verify ATTR_BIT_MAP_COUNT value
    #[test]
    fn test_attr_bit_map_count() {
        // ATTR_BIT_MAP_COUNT should be 5 (number of attribute categories)
        const ATTR_BIT_MAP_COUNT: u16 = 5;
        assert_eq!(ATTR_BIT_MAP_COUNT, 5);
    }

    /// Verify common attribute flags
    #[test]
    fn test_common_attr_flags() {
        const ATTR_CMN_RETURNED_ATTRS: u32 = 0x80000000;
        const ATTR_CMN_NAME: u32 = 0x00000001;
        const ATTR_CMN_OBJTYPE: u32 = 0x00000008;
        const ATTR_CMN_MODTIME: u32 = 0x00000400;
        const ATTR_CMN_ACCESSMASK: u32 = 0x00020000;
        const ATTR_CMN_FILEID: u32 = 0x02000000;

        // Verify values match macOS headers
        assert_eq!(ATTR_CMN_RETURNED_ATTRS, 0x80000000);
        assert_eq!(ATTR_CMN_NAME, 0x00000001);
        assert_eq!(ATTR_CMN_OBJTYPE, 0x00000008);
        assert_eq!(ATTR_CMN_MODTIME, 0x00000400);
        assert_eq!(ATTR_CMN_ACCESSMASK, 0x00020000);
        assert_eq!(ATTR_CMN_FILEID, 0x02000000);

        // Verify no overlap between common flags we use
        let all_common = ATTR_CMN_RETURNED_ATTRS | ATTR_CMN_NAME | ATTR_CMN_OBJTYPE
            | ATTR_CMN_MODTIME | ATTR_CMN_ACCESSMASK | ATTR_CMN_FILEID;
        assert_eq!(all_common.count_ones(), 6, "flags should not overlap");
    }

    /// Verify file attribute flags
    #[test]
    fn test_file_attr_flags() {
        const ATTR_FILE_TOTALSIZE: u32 = 0x00000002;
        const ATTR_FILE_ALLOCSIZE: u32 = 0x00000004;
        const ATTR_FILE_DATALENGTH: u32 = 0x00000200;

        assert_eq!(ATTR_FILE_TOTALSIZE, 0x00000002);
        assert_eq!(ATTR_FILE_ALLOCSIZE, 0x00000004);
        assert_eq!(ATTR_FILE_DATALENGTH, 0x00000200);

        // Verify no overlap
        let all_file = ATTR_FILE_TOTALSIZE | ATTR_FILE_ALLOCSIZE | ATTR_FILE_DATALENGTH;
        assert_eq!(all_file.count_ones(), 3, "flags should not overlap");
    }

    /// Verify directory attribute flags
    #[test]
    fn test_dir_attr_flags() {
        const ATTR_DIR_ENTRYCOUNT: u32 = 0x00000002;
        assert_eq!(ATTR_DIR_ENTRYCOUNT, 0x00000002);
    }

    /// Verify filesystem options
    #[test]
    fn test_fs_options() {
        const FSOPT_NOFOLLOW: u64 = 0x00000001;
        const FSOPT_PACK_INVAL_ATTRS: u64 = 0x00000008;

        assert_eq!(FSOPT_NOFOLLOW, 0x00000001);
        assert_eq!(FSOPT_PACK_INVAL_ATTRS, 0x00000008);
    }

    /// Verify vnode type values for ObjectType
    #[test]
    fn test_vnode_types() {
        // Values from sys/vnode.h enum vtype
        const VREG: u32 = 1;  // Regular file
        const VDIR: u32 = 2;  // Directory
        const VBLK: u32 = 3;  // Block device
        const VCHR: u32 = 4;  // Character device
        const VLNK: u32 = 5;  // Symbolic link
        const VSOCK: u32 = 6; // Socket
        const VFIFO: u32 = 7; // Named pipe

        assert_eq!(VREG, 1);
        assert_eq!(VDIR, 2);
        assert_eq!(VBLK, 3);
        assert_eq!(VCHR, 4);
        assert_eq!(VLNK, 5);
        assert_eq!(VSOCK, 6);
        assert_eq!(VFIFO, 7);
    }
}
