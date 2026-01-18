# getattrlistbulk Crate Specification

## Overview

This crate provides safe Rust bindings for the macOS `getattrlistbulk()` system call, enabling high-performance directory enumeration with bulk metadata retrieval in a single syscall.

## Purpose

Traditional directory enumeration requires:
1. `opendir()` / `readdir()` to list entries
2. `stat()` per entry to get metadata

This results in N+1 syscalls for N files. `getattrlistbulk()` retrieves both directory entries AND their metadata in a single syscall, significantly reducing kernel transitions.

## Target Platform

- **Operating System**: macOS only (Darwin kernel)
- **Architectures**: x86_64-apple-darwin, aarch64-apple-darwin
- **Minimum macOS Version**: 10.10 (Yosemite) - when `getattrlistbulk()` was introduced

## System Call Reference

### C Function Signature

```c
#include <sys/attr.h>

ssize_t getattrlistbulk(
    int dirfd,                    // Open directory file descriptor
    struct attrlist *alist,       // Attributes to retrieve
    void *attributeBuffer,        // Output buffer
    size_t bufferSize,            // Buffer size in bytes
    uint64_t options              // Option flags
);
```

### Return Value

- **> 0**: Number of entries returned in buffer
- **0**: No more entries (enumeration complete)
- **-1**: Error (check errno)

### Key Constants

```c
// Attribute groups (attrlist.bitmapcount)
#define ATTR_BIT_MAP_COUNT 5

// Common attributes (attrlist.commonattr)
#define ATTR_CMN_RETURNED_ATTRS  0x80000000
#define ATTR_CMN_NAME            0x00000001
#define ATTR_CMN_OBJTYPE         0x00000008
#define ATTR_CMN_MODTIME         0x00000400
#define ATTR_CMN_ACCESSMASK      0x00020000
#define ATTR_CMN_FILEID          0x02000000

// File attributes (attrlist.fileattr)
#define ATTR_FILE_TOTALSIZE      0x00000002
#define ATTR_FILE_ALLOCSIZE      0x00000004
#define ATTR_FILE_DATALENGTH     0x00000200

// Directory attributes (attrlist.dirattr)
#define ATTR_DIR_ENTRYCOUNT      0x00000002

// Options
#define FSOPT_NOFOLLOW           0x00000001
#define FSOPT_PACK_INVAL_ATTRS   0x00000008
```

### Buffer Format

Each entry in the returned buffer has the following structure:

```
+------------------+
| length (u32)     |  Total length of this entry
+------------------+
| attribute_set    |  Which attributes are actually present (if ATTR_CMN_RETURNED_ATTRS)
+------------------+
| fixed attrs      |  Fixed-size attributes in order specified
+------------------+
| attrreference    |  For variable-length attrs: offset (i32) + length (u32)
+------------------+
| variable data    |  Variable-length data (names, etc.) at end of entry
+------------------+
```

**Critical**: Entries are NOT fixed size. Each entry's length field indicates where the next entry begins.

### Attribute Order in Buffer

When attributes are present, they appear in this exact order:

| Order | Attribute | Size | Condition |
|-------|-----------|------|-----------|
| 1 | entry_length | 4 bytes (u32) | Always present |
| 2 | attribute_set | 20 bytes | Always (ATTR_CMN_RETURNED_ATTRS) |
| 3 | name | 8 bytes (attrreference) | If ATTR_CMN_NAME |
| 4 | objtype | 4 bytes (u32) | If ATTR_CMN_OBJTYPE |
| 5 | modtime | 16 bytes (timespec) | If ATTR_CMN_MODTIME |
| 6 | accessmask | 4 bytes (u32) | If ATTR_CMN_ACCESSMASK |
| 7 | fileid | 8 bytes (u64) | If ATTR_CMN_FILEID |
| 8 | totalsize | 8 bytes (u64) | If ATTR_FILE_TOTALSIZE |
| 9 | allocsize | 8 bytes (u64) | If ATTR_FILE_ALLOCSIZE |
| 10 | entrycount | 4 bytes (u32) | If ATTR_DIR_ENTRYCOUNT |
| - | variable data | varies | Name strings at end |

### timespec Structure

```c
struct timespec {
    int64_t tv_sec;   // Seconds since Unix epoch
    int64_t tv_nsec;  // Nanoseconds (0-999999999)
};
```
Total size: 16 bytes on 64-bit systems.

### attrreference Offset Semantics

The `attr_dataoffset` field in `attrreference` is **relative to the location of the attrreference struct itself**, not relative to the entry start.

```
To find name string:
  name_ptr = &attrreference + attrreference.attr_dataoffset
```

### Alignment

Attributes are naturally aligned:
- 4-byte values aligned to 4-byte boundaries
- 8-byte values aligned to 8-byte boundaries
- Padding bytes may appear between attributes to maintain alignment

## Rust API Design

### Core Types

```rust
/// Attributes that can be requested for each directory entry
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestedAttributes {
    pub name: bool,           // File/directory name
    pub object_type: bool,    // File, directory, symlink, etc.
    pub size: bool,           // Total size in bytes
    pub alloc_size: bool,     // Allocated size on disk
    pub modified_time: bool,  // Last modification time
    pub permissions: bool,    // Unix permissions mask
    pub inode: bool,          // File ID / inode number
    pub entry_count: bool,    // Number of entries (directories only)
}

/// Object type returned by the filesystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Regular,
    Directory,
    Symlink,
    BlockDevice,
    CharDevice,
    Socket,
    Fifo,
    Unknown(u32),
}
```

### ObjectType Values

| Variant | vnode.h Value | Description |
|---------|---------------|-------------|
| Regular | 1 (VREG) | Regular file |
| Directory | 2 (VDIR) | Directory |
| BlockDevice | 3 (VBLK) | Block special device |
| CharDevice | 4 (VCHR) | Character special device |
| Symlink | 5 (VLNK) | Symbolic link |
| Socket | 6 (VSOCK) | Socket |
| Fifo | 7 (VFIFO) | Named pipe |

```rust

/// Metadata for a single directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub object_type: Option<ObjectType>,
    pub size: Option<u64>,
    pub alloc_size: Option<u64>,
    pub modified_time: Option<std::time::SystemTime>,
    pub permissions: Option<u32>,
    pub inode: Option<u64>,
    pub entry_count: Option<u32>,  // Only for directories
}

/// Iterator over directory entries
pub struct DirEntries {
    // Internal state
}

impl Iterator for DirEntries {
    type Item = Result<DirEntry, Error>;
}

/// Error type for this crate
#[derive(Debug)]
pub enum Error {
    /// Failed to open directory
    Open(std::io::Error),
    /// System call failed
    Syscall(std::io::Error),
    /// Buffer parsing error
    Parse(String),
    /// Platform not supported
    NotSupported,
}
```

### Primary API

```rust
/// Read all entries from a directory with requested attributes
pub fn read_dir<P: AsRef<Path>>(
    path: P,
    attrs: RequestedAttributes,
) -> Result<DirEntries, Error>;

/// Read entries with custom buffer size (default: 64KB)
pub fn read_dir_with_buffer<P: AsRef<Path>>(
    path: P,
    attrs: RequestedAttributes,
    buffer_size: usize,
) -> Result<DirEntries, Error>;

/// Low-level: read into provided buffer, returns entry count
/// For users who want to manage their own memory
pub unsafe fn read_dir_raw(
    dirfd: RawFd,
    attrs: &RequestedAttributes,
    buffer: &mut [u8],
) -> Result<(usize, usize), Error>;  // (entry_count, bytes_used)
```

### Builder Pattern (Optional)

```rust
let entries = DirReader::new("/path/to/dir")
    .attributes(RequestedAttributes {
        name: true,
        size: true,
        object_type: true,
        ..Default::default()
    })
    .buffer_size(128 * 1024)
    .follow_symlinks(false)
    .read()?;

for entry in entries {
    let entry = entry?;
    println!("{}: {} bytes", entry.name, entry.size.unwrap_or(0));
}
```

## Implementation Requirements

### IR-1: FFI Declarations

File: `src/ffi.rs`

Must declare:
- `getattrlistbulk()` function
- `struct attrlist`
- All `ATTR_*` constants
- `FSOPT_*` constants
- `struct attribute_set` (for ATTR_CMN_RETURNED_ATTRS)

### IR-2: Type Conversions

File: `src/types.rs`

Must implement:
- `RequestedAttributes` → `attrlist` conversion
- Buffer parsing into `DirEntry` structs
- `ObjectType` from raw `VTYPE` values
- Timestamp conversion to `std::time::SystemTime`

### IR-3: Buffer Parser

File: `src/parser.rs`

Must handle:
- Variable-length entry parsing
- `attrreference` offset/length resolution for names
- Optional attribute presence (ATTR_CMN_RETURNED_ATTRS)
- Alignment requirements (attributes may be padded)
- UTF-8 validation for filenames

### IR-4: Iterator Implementation

File: `src/iter.rs`

Must implement:
- Lazy iteration (parse entries on-demand)
- Automatic buffer refill when exhausted
- Proper cleanup on drop (close directory fd)
- `Send` but not `Sync` (contains file descriptor)

### IR-5: Error Handling

File: `src/error.rs`

Must handle these errno values:
- `ENOTDIR`: Path is not a directory
- `EACCES`: Permission denied
- `ENOENT`: Path does not exist
- `EINVAL`: Invalid attributes or options
- `EIO`: I/O error
- `EINTR`: Interrupted (should retry)

### IR-6: Safety Invariants

The public API must be safe. Unsafe code is allowed internally but must:
- Never return uninitialized memory
- Never create invalid UTF-8 strings (use lossy conversion)
- Never leak file descriptors
- Handle buffer overflow attempts
- Validate all offsets before dereferencing

## Performance Requirements

### PR-1: Syscall Efficiency

- One `getattrlistbulk()` call per buffer fill
- Buffer size should be configurable (default 64KB)
- Larger buffers = fewer syscalls = faster for large directories

### PR-2: Memory Efficiency

- Parse entries lazily, not all at once
- Reuse buffer across iterations
- `DirEntry` should be reasonably sized (< 200 bytes)

### PR-3: Benchmarks

Must include benchmarks comparing against:
- `std::fs::read_dir()` + `metadata()` per file
- Raw `readdir()` + `stat()` via libc

Target: 2-5x faster than std::fs for directories with 1000+ files.

## Testing Requirements

### TR-1: Unit Tests

- `attrlist` construction from `RequestedAttributes`
- Buffer parsing with known byte sequences
- Error conversion from errno values
- ObjectType mapping

### TR-2: Integration Tests

- Read actual directories on macOS
- Verify metadata matches `std::fs::metadata()`
- Handle permission errors gracefully
- Handle symlinks correctly
- Handle empty directories
- Handle directories with 10,000+ entries

### TR-3: Edge Cases

- Filenames with unicode characters
- Filenames with null bytes (should error or skip)
- Very long filenames (up to NAME_MAX)
- Directories being modified during iteration
- Insufficient permissions

### TR-4: Platform Tests

- Compile fails gracefully on non-macOS
- Compiles on both x86_64 and aarch64

## Documentation Requirements

### DR-1: Crate-level Documentation

- Overview and motivation
- Platform requirements
- Quick start example
- Comparison with alternatives

### DR-2: API Documentation

- All public types fully documented
- All public functions with examples
- Safety requirements for unsafe functions
- Error conditions documented

### DR-3: README

- Badge for crates.io, docs.rs
- Installation instructions
- Basic usage example
- Performance comparison
- License information

## Versioning

- Start at 0.1.0
- Follow SemVer strictly
- 1.0.0 when API is stable and battle-tested

## License

Dual-licensed under MIT OR Apache-2.0, standard for Rust ecosystem.
