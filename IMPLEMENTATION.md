# Implementation Guide for Agents

This document defines the implementation tasks for the `getattrlistbulk` crate, structured for parallel agent development, cross-auditing, and systematic testing.

## Architecture Overview

```
src/
├── lib.rs          # Public API re-exports, crate docs
├── ffi.rs          # FFI declarations (Task A)
├── types.rs        # Rust types and conversions (Task B)
├── parser.rs       # Buffer parsing logic (Task C)
├── iter.rs         # Iterator implementation (Task D)
├── error.rs        # Error types (Task E)
└── builder.rs      # Builder pattern API (Task F)

tests/
├── ffi_tests.rs           # FFI struct/constant validation
├── parser_tests.rs        # Parser behavior tests with real files
└── integration_tests.rs   # Full integration tests (includes edge cases)

benches/
└── traversal.rs    # Performance benchmarks
```

---

## Task Definitions

### Task A: FFI Declarations

**File**: `src/ffi.rs`
**Dependencies**: None
**Parallel**: Yes - can be developed independently
**Estimated effort**: 30 minutes

#### Requirements

1. Declare the `getattrlistbulk` function:
```rust
extern "C" {
    pub fn getattrlistbulk(
        dirfd: libc::c_int,
        alist: *mut attrlist,
        attribute_buffer: *mut libc::c_void,
        buffer_size: libc::size_t,
        options: u64,
    ) -> libc::ssize_t;
}
```

2. Define `attrlist` struct:
```rust
#[repr(C)]
pub struct attrlist {
    pub bitmapcount: u16,    // Always ATTR_BIT_MAP_COUNT (5)
    pub reserved: u16,       // Always 0
    pub commonattr: u32,     // ATTR_CMN_* flags
    pub volattr: u32,        // Volume attributes (unused for files)
    pub dirattr: u32,        // ATTR_DIR_* flags
    pub fileattr: u32,       // ATTR_FILE_* flags
    pub forkattr: u32,       // Fork attributes (unused)
}
```

3. Define all constants using `bitflags!`:
```rust
bitflags! {
    pub struct CommonAttr: u32 {
        const RETURNED_ATTRS = 0x80000000;
        const NAME = 0x00000001;
        const OBJTYPE = 0x00000008;
        const MODTIME = 0x00000400;
        const ACCESSMASK = 0x00020000;
        const FILEID = 0x02000000;
    }

    pub struct FileAttr: u32 {
        const TOTALSIZE = 0x00000002;
        const ALLOCSIZE = 0x00000004;
        const DATALENGTH = 0x00000200;
    }

    pub struct DirAttr: u32 {
        const ENTRYCOUNT = 0x00000002;
    }

    pub struct FsOptions: u64 {
        const NOFOLLOW = 0x00000001;
        const PACK_INVAL_ATTRS = 0x00000008;
    }
}
```

4. Define `attribute_set` for returned attributes:
```rust
#[repr(C)]
pub struct attribute_set {
    pub commonattr: u32,
    pub volattr: u32,
    pub dirattr: u32,
    pub fileattr: u32,
    pub forkattr: u32,
}
```

5. Define `attrreference` for variable-length data:
```rust
#[repr(C)]
pub struct attrreference {
    pub attr_dataoffset: i32,
    pub attr_length: u32,
}
```

6. Define `ATTR_BIT_MAP_COUNT`:
```rust
pub const ATTR_BIT_MAP_COUNT: u16 = 5;
```

#### Verification Criteria

- [ ] Compiles with `cargo check` on macOS
- [ ] All structs have correct `#[repr(C)]`
- [ ] All constants match values in `/usr/include/sys/attr.h`
- [ ] `bitflags` derive works correctly

#### Cross-Audit Checklist (for reviewing agent)

- [ ] Verify struct sizes match C equivalents
- [ ] Verify constant values against macOS headers
- [ ] Check alignment/padding requirements
- [ ] Ensure no missing attributes that parser needs

---

### Task B1: Type Definitions (Structs Only)

**File**: `src/types.rs` (struct definitions)
**Dependencies**: None
**Parallel**: Yes - fully independent
**Estimated effort**: 15 minutes

#### Requirements

Define these structs WITHOUT conversion implementations:
- `RequestedAttributes` struct with all boolean fields
- `ObjectType` enum with all variants
- `DirEntry` struct with all fields

#### Deliverables
- All public type definitions
- Builder methods on RequestedAttributes (with_name, with_size, etc.)
- Helper methods on DirEntry (is_dir, is_file, is_symlink)

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestedAttributes {
    pub name: bool,
    pub object_type: bool,
    pub size: bool,
    pub alloc_size: bool,
    pub modified_time: bool,
    pub permissions: bool,
    pub inode: bool,
    pub entry_count: bool,
}

impl RequestedAttributes {
    pub fn all() -> Self { /* all fields true */ }

    // Builder methods
    pub fn with_name(mut self) -> Self { self.name = true; self }
    pub fn with_size(mut self) -> Self { self.size = true; self }
    // ... etc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Regular,      // VREG
    Directory,    // VDIR
    Symlink,      // VLNK
    BlockDevice,  // VBLK
    CharDevice,   // VCHR
    Socket,       // VSOCK
    Fifo,         // VFIFO
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub object_type: Option<ObjectType>,
    pub size: Option<u64>,
    pub alloc_size: Option<u64>,
    pub modified_time: Option<std::time::SystemTime>,
    pub permissions: Option<u32>,
    pub inode: Option<u64>,
    pub entry_count: Option<u32>,
}

impl DirEntry {
    pub fn is_dir(&self) -> bool {
        matches!(self.object_type, Some(ObjectType::Directory))
    }
    pub fn is_file(&self) -> bool {
        matches!(self.object_type, Some(ObjectType::Regular))
    }
    pub fn is_symlink(&self) -> bool {
        matches!(self.object_type, Some(ObjectType::Symlink))
    }
}
```

#### Verification Criteria

- [ ] `RequestedAttributes::all()` sets all flags correctly
- [ ] ObjectType covers all vnode types
- [ ] Builder methods chain correctly
- [ ] DirEntry helper methods work correctly

#### Cross-Audit Checklist

- [ ] All vnode type values verified against sys/vnode.h
- [ ] DirEntry fields align with SPECIFICATION.md

---

### Task B2: Type Conversions

**File**: `src/types.rs` (conversion implementations)
**Dependencies**: Task A (ffi.rs), Task B1
**Parallel**: No - requires FFI types
**Estimated effort**: 15 minutes

#### Requirements

Implement conversions:
- `From<RequestedAttributes> for ffi::attrlist`
- `From<u32> for ObjectType` (vnode type conversion)

```rust
impl From<RequestedAttributes> for ffi::attrlist {
    fn from(req: RequestedAttributes) -> Self {
        let mut common = ffi::CommonAttr::RETURNED_ATTRS;
        let mut file = ffi::FileAttr::empty();
        let mut dir = ffi::DirAttr::empty();

        if req.name { common |= ffi::CommonAttr::NAME; }
        if req.object_type { common |= ffi::CommonAttr::OBJTYPE; }
        if req.size { file |= ffi::FileAttr::TOTALSIZE; }
        // ... etc

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

impl From<u32> for ObjectType {
    fn from(vtype: u32) -> Self {
        match vtype {
            1 => ObjectType::Regular,
            2 => ObjectType::Directory,
            // ... reference sys/vnode.h for values
            v => ObjectType::Unknown(v),
        }
    }
}
```

#### Verification Criteria

- [ ] `attrlist` conversion produces correct bitmaps
- [ ] ObjectType From<u32> handles all vnode types

#### Cross-Audit Checklist

- [ ] attrlist conversion matches attribute order expected by parser
- [ ] Vnode type constants verified against sys/vnode.h

---

### Task C: Buffer Parser

**File**: `src/parser.rs`
**Dependencies**: Task A (ffi.rs), Task B (types.rs)
**Parallel**: No - needs A and B first
**Estimated effort**: 45 minutes
**Complexity**: HIGH - most error-prone component

#### Requirements

1. Define parser state:
```rust
pub struct BufferParser<'a> {
    buffer: &'a [u8],
    offset: usize,
    requested: RequestedAttributes,
}
```

2. Implement entry parsing:
```rust
impl<'a> BufferParser<'a> {
    pub fn new(buffer: &'a [u8], requested: RequestedAttributes) -> Self;

    /// Parse next entry, return None if buffer exhausted
    pub fn next_entry(&mut self) -> Option<Result<DirEntry, ParseError>>;

    /// Reset parser for new buffer contents
    pub fn reset(&mut self, buffer: &'a [u8]);
}
```

3. Parse fixed attributes in order:
```
Order of attributes in buffer (when requested):
1. attribute_set (if ATTR_CMN_RETURNED_ATTRS) - 20 bytes
2. name (attrreference) - 8 bytes (offset + length)
3. objtype (u32) - 4 bytes
4. modtime (timespec) - 16 bytes
5. accessmask (u32) - 4 bytes
6. fileid (u64) - 8 bytes
7. totalsize (u64) - 8 bytes (file only)
8. allocsize (u64) - 8 bytes (file only)
9. entrycount (u32) - 4 bytes (dir only)
```

4. Handle `attrreference` for names:
```rust
fn parse_name(&self, entry_start: usize, ref_offset: usize) -> Result<String, ParseError> {
    let attr_ref: attrreference = self.read_at(ref_offset)?;

    // Offset is relative to the attrreference location itself
    let name_start = ref_offset + attr_ref.attr_dataoffset as usize;
    let name_end = name_start + attr_ref.attr_length as usize - 1; // -1 for null terminator

    // Bounds check
    if name_end > entry_start + entry_length {
        return Err(ParseError::InvalidOffset);
    }

    // Extract and validate UTF-8
    let name_bytes = &self.buffer[name_start..name_end];
    String::from_utf8(name_bytes.to_vec())
        .or_else(|_| Ok(String::from_utf8_lossy(name_bytes).into_owned()))
}
```

5. Handle returned_attrs to know which attributes are actually present:
```rust
fn parse_entry(&mut self, entry_start: usize, entry_length: u32) -> Result<DirEntry, ParseError> {
    let mut offset = entry_start + 4; // Skip length field

    // Read which attributes were actually returned
    let returned: attribute_set = self.read_at(offset)?;
    offset += std::mem::size_of::<attribute_set>();

    // Now parse only the attributes that are present
    let name = if returned.commonattr & CommonAttr::NAME.bits() != 0 {
        Some(self.parse_name(entry_start, offset)?)
        offset += 8; // attrreference size
    } else {
        None
    };

    // ... continue for each attribute
}
```

6. Ensure alignment handling:
```rust
fn align_offset(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
}
```

#### Verification Criteria

- [ ] Correctly parses entries with all attributes
- [ ] Correctly parses entries with subset of attributes
- [ ] Handles attrreference offsets correctly
- [ ] Produces valid UTF-8 strings (lossy if needed)
- [ ] Returns error on truncated buffer
- [ ] Returns error on invalid offsets

#### Cross-Audit Checklist

- [ ] Attribute order matches macOS documentation
- [ ] All bounds checks present before buffer access
- [ ] No panics on malformed input
- [ ] Alignment handling matches kernel behavior
- [ ] attrreference offset calculation is correct (relative to ref location)

---

### Task D: Iterator Implementation

**File**: `src/iter.rs`
**Dependencies**: Task A, B, C
**Parallel**: No - needs parser
**Estimated effort**: 30 minutes

#### Requirements

1. Define iterator struct:
```rust
pub struct DirEntries {
    dirfd: RawFd,
    buffer: Vec<u8>,
    parser: BufferParser<'static>, // Uses unsafe lifetime extension
    requested: RequestedAttributes,
    exhausted: bool,
}
```

2. Implement creation:
```rust
impl DirEntries {
    pub(crate) fn new(path: &Path, requested: RequestedAttributes, buffer_size: usize) -> Result<Self, Error> {
        let dirfd = open_directory(path)?;
        let buffer = vec![0u8; buffer_size];

        Ok(Self {
            dirfd,
            buffer,
            parser: BufferParser::new(&[], requested),
            requested,
            exhausted: false,
        })
    }

    fn refill_buffer(&mut self) -> Result<bool, Error> {
        let mut attrlist: ffi::attrlist = self.requested.into();

        let result = unsafe {
            ffi::getattrlistbulk(
                self.dirfd,
                &mut attrlist,
                self.buffer.as_mut_ptr() as *mut libc::c_void,
                self.buffer.len(),
                ffi::FsOptions::PACK_INVAL_ATTRS.bits(),
            )
        };

        if result < 0 {
            return Err(Error::Syscall(std::io::Error::last_os_error()));
        }

        if result == 0 {
            self.exhausted = true;
            return Ok(false);
        }

        // Update parser with new buffer contents
        // Need unsafe lifetime extension here
        self.parser.reset(unsafe {
            std::slice::from_raw_parts(self.buffer.as_ptr(), /* bytes used */)
        });

        Ok(true)
    }
}
```

3. Implement Iterator trait:
```rust
impl Iterator for DirEntries {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to get next entry from current buffer
            if let Some(result) = self.parser.next_entry() {
                return Some(result.map_err(Error::Parse));
            }

            // Buffer exhausted, try to refill
            if self.exhausted {
                return None;
            }

            match self.refill_buffer() {
                Ok(true) => continue,  // Got more entries
                Ok(false) => return None,  // No more entries
                Err(e) => return Some(Err(e)),
            }
        }
    }
}
```

4. Implement Drop to close directory:
```rust
impl Drop for DirEntries {
    fn drop(&mut self) {
        unsafe { libc::close(self.dirfd); }
    }
}
```

5. Implement Send (but not Sync):
```rust
// DirEntries owns the fd exclusively, safe to send between threads
unsafe impl Send for DirEntries {}
```

#### Verification Criteria

- [ ] Correctly iterates through all entries
- [ ] Refills buffer automatically
- [ ] Closes fd on drop
- [ ] Handles errors during iteration
- [ ] Is Send but not Sync

#### Cross-Audit Checklist

- [ ] No fd leaks on error paths
- [ ] Buffer lifetime handling is sound
- [ ] EINTR is retried (or documented as not)
- [ ] Memory safety of lifetime extension is justified

---

### Task E: Error Types

**File**: `src/error.rs`
**Dependencies**: None
**Parallel**: Yes
**Estimated effort**: 15 minutes

#### Requirements

1. Define error enum:
```rust
#[derive(Debug)]
pub enum Error {
    Open(std::io::Error),
    Syscall(std::io::Error),
    Parse(String),
    NotSupported,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Open(e) => write!(f, "failed to open directory: {}", e),
            Error::Syscall(e) => write!(f, "getattrlistbulk failed: {}", e),
            Error::Parse(msg) => write!(f, "buffer parse error: {}", msg),
            Error::NotSupported => write!(f, "getattrlistbulk is only supported on macOS"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Open(e) | Error::Syscall(e) => Some(e),
            _ => None,
        }
    }
}
```

2. Define parse error (internal):
```rust
#[derive(Debug)]
pub(crate) enum ParseError {
    BufferTooSmall,
    InvalidOffset,
    InvalidUtf8,
    UnexpectedEnd,
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::Parse(format!("{:?}", e))
    }
}
```

#### Verification Criteria

- [ ] Error implements std::error::Error
- [ ] Display messages are user-friendly
- [ ] source() returns underlying io::Error where applicable

---

### Task F: Builder Pattern API

**File**: `src/builder.rs`
**Dependencies**: Task D (iter.rs)
**Parallel**: Partially - interface can be designed independently
**Estimated effort**: 20 minutes

#### Requirements

1. Define builder:
```rust
pub struct DirReader {
    path: PathBuf,
    attrs: RequestedAttributes,
    buffer_size: usize,
    follow_symlinks: bool,
}

impl DirReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            attrs: RequestedAttributes::default(),
            buffer_size: 64 * 1024,
            follow_symlinks: true,
        }
    }

    pub fn name(mut self) -> Self {
        self.attrs.name = true;
        self
    }

    pub fn size(mut self) -> Self {
        self.attrs.size = true;
        self
    }

    // ... all attribute methods

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    pub fn read(self) -> Result<DirEntries, Error> {
        DirEntries::new(&self.path, self.attrs, self.buffer_size)
    }
}
```

#### Verification Criteria

- [ ] Builder methods chain fluently
- [ ] Default buffer size is reasonable (64KB)
- [ ] All attributes accessible via builder

---

### Task G: Public API (lib.rs)

**File**: `src/lib.rs`
**Dependencies**: All other tasks
**Parallel**: No - final integration
**Estimated effort**: 15 minutes

#### Requirements

1. Crate-level documentation
2. Re-export public types
3. Define convenience functions:

```rust
//! # getattrlistbulk
//!
//! Safe Rust bindings for the macOS `getattrlistbulk()` system call.
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
//!     println!("{}", entry.unwrap().name);
//! }
//! ```

#![cfg(target_os = "macos")]

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

/// Read directory entries with specified attributes.
pub fn read_dir<P: AsRef<std::path::Path>>(
    path: P,
    attrs: RequestedAttributes,
) -> Result<DirEntries, Error> {
    read_dir_with_buffer(path, attrs, 64 * 1024)
}

/// Read directory entries with custom buffer size.
pub fn read_dir_with_buffer<P: AsRef<std::path::Path>>(
    path: P,
    attrs: RequestedAttributes,
    buffer_size: usize,
) -> Result<DirEntries, Error> {
    DirEntries::new(path.as_ref(), attrs, buffer_size)
}
```

4. Add compile-time check for non-macOS:

```rust
#[cfg(not(target_os = "macos"))]
compile_error!("getattrlistbulk is only available on macOS");
```

---

## Testing Tasks

### Task T1: FFI Unit Tests

**File**: `tests/ffi_tests.rs`
**Dependencies**: Task A
**Estimated effort**: 15 minutes

```rust
#[test]
fn test_attrlist_size() {
    assert_eq!(std::mem::size_of::<ffi::attrlist>(), 24);
}

#[test]
fn test_attrreference_size() {
    assert_eq!(std::mem::size_of::<ffi::attrreference>(), 8);
}

#[test]
fn test_common_attr_flags() {
    assert_eq!(ffi::CommonAttr::NAME.bits(), 0x00000001);
    assert_eq!(ffi::CommonAttr::OBJTYPE.bits(), 0x00000008);
    // ... verify all constants
}
```

### Task T2: Parser Unit Tests

**File**: `tests/parser_tests.rs`
**Dependencies**: Task C
**Estimated effort**: 30 minutes

Create known byte sequences and verify parsing:

```rust
#[test]
fn test_parse_single_entry() {
    // Construct a valid buffer with known values
    let buffer = construct_test_buffer(&[
        TestEntry { name: "file.txt", size: 1234, ... }
    ]);

    let mut parser = BufferParser::new(&buffer, RequestedAttributes::all());
    let entry = parser.next_entry().unwrap().unwrap();

    assert_eq!(entry.name, "file.txt");
    assert_eq!(entry.size, Some(1234));
}

#[test]
fn test_parse_multiple_entries() { ... }

#[test]
fn test_parse_unicode_names() { ... }

#[test]
fn test_parse_truncated_buffer() { ... }
```

### Task T3: Integration Tests

**File**: `tests/integration_tests.rs`
**Dependencies**: All tasks
**Estimated effort**: 30 minutes

```rust
#[test]
fn test_read_tmp_directory() {
    let entries: Vec<_> = read_dir("/tmp", RequestedAttributes::all())
        .unwrap()
        .collect();

    // /tmp always has something
    assert!(!entries.is_empty());
}

#[test]
fn test_metadata_matches_std_fs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

    let entries: Vec<_> = read_dir(dir.path(), RequestedAttributes::all())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let entry = entries.iter().find(|e| e.name == "test.txt").unwrap();
    let std_meta = std::fs::metadata(dir.path().join("test.txt")).unwrap();

    assert_eq!(entry.size.unwrap(), std_meta.len());
}

#[test]
fn test_permission_denied() {
    let result = read_dir("/private/var/root", RequestedAttributes::default());
    assert!(matches!(result, Err(Error::Open(_))));
}
```

### Task T4: Benchmark

**File**: `benches/traversal.rs`
**Dependencies**: All tasks
**Estimated effort**: 20 minutes

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_getattrlistbulk(c: &mut Criterion) {
    c.bench_function("getattrlistbulk /usr/lib", |b| {
        b.iter(|| {
            let entries: Vec<_> = read_dir("/usr/lib", RequestedAttributes::all())
                .unwrap()
                .collect();
            entries.len()
        })
    });
}

fn bench_std_fs(c: &mut Criterion) {
    c.bench_function("std::fs /usr/lib", |b| {
        b.iter(|| {
            let entries: Vec<_> = std::fs::read_dir("/usr/lib")
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| {
                    let meta = e.metadata().ok();
                    (e.file_name(), meta.map(|m| m.len()))
                })
                .collect();
            entries.len()
        })
    });
}

criterion_group!(benches, bench_getattrlistbulk, bench_std_fs);
criterion_main!(benches);
```

---

## Parallel Execution Strategy

### Phase 1: Independent Tasks (Parallel)

Execute simultaneously:
- **Agent 1**: Task A (ffi.rs)
- **Agent 2**: Task B interface (types.rs - struct definitions only)
- **Agent 3**: Task E (error.rs)

### Phase 2: Core Implementation (Sequential with Audit)

After Phase 1 completion:
- **Agent 1**: Task B completion (types.rs - conversions)
- **Agent 2**: Audit Task A output

Then:
- **Agent 1**: Task C (parser.rs) - **CRITICAL PATH**
- **Agent 2**: Audit Task B, prepare Task D interface

### Phase 3: Integration (Sequential)

- **Agent 1**: Task D (iter.rs)
- **Agent 2**: Audit Task C (parser is highest-risk)

### Phase 4: Polish (Parallel)

- **Agent 1**: Task F (builder.rs), Task G (lib.rs)
- **Agent 2**: Task T1, T2 (tests)

### Phase 5: Validation

- **Agent 1**: Task T3, T4 (integration tests, benchmarks)
- **Agent 2**: Full cross-audit of all code

---

## Cross-Audit Protocol

When auditing another agent's work:

1. **Compile Check**: `cargo check` must pass
2. **Constant Verification**: Compare all constants against `/usr/include/sys/attr.h`
3. **Bounds Checking**: Every buffer access must have bounds check
4. **Error Paths**: Every error must be handled, no panics on invalid input
5. **Memory Safety**: All unsafe code must have safety comments
6. **Documentation**: Public items must have doc comments

### Severity Definitions

| Severity | Definition | Examples | Action |
|----------|------------|----------|--------|
| **Critical** | Memory safety issue, soundness bug, wrong FFI constant that causes undefined behavior | Wrong struct size, buffer overflow, incorrect constant value | Must fix before merge |
| **Major** | Logic error, missing error handling, incorrect behavior | Missing bounds check, wrong calculation, unhandled error case | Should fix before merge |
| **Minor** | Style issue, suboptimal code, documentation gap | Unused variable, missing doc comment, non-idiomatic code | Can fix later |

### Pass/Fail Criteria

- **PASS**: Zero critical issues, zero major issues
- **CONDITIONAL PASS**: Zero critical issues, 1-2 major issues with documented fix plan
- **FAIL**: Any critical issue, OR 3+ major issues

### Audit Timing

1. **Task Completion Audit**: Immediately after each task is marked complete
2. **Phase Gate Audit**: Before moving from one phase to the next
3. **Integration Audit**: After Task G (lib.rs) combines all modules
4. **Final Audit**: Before marking crate as release-ready

### Audit Report Format

```markdown
## Audit: [task_name]
**Auditor**: [agent_id]
**Date**: [timestamp]

### Verification
- [ ] Compiles without warnings
- [ ] Constants verified against headers
- [ ] Bounds checks present
- [ ] Error handling complete
- [ ] Unsafe code justified

### Issues Found
1. [description] - [severity: critical/major/minor]

### Recommendations
1. [suggestion]

### Approval
[ ] Approved / [ ] Needs revision
```

---

## Completion Criteria

The crate is complete when:

1. [ ] All tasks A-G implemented
2. [ ] All tests T1-T4 passing
3. [ ] Cross-audit completed with no critical issues
4. [ ] `cargo clippy` clean
5. [ ] `cargo doc` generates complete documentation
6. [ ] Benchmark shows >= 2x improvement over std::fs
7. [ ] README examples work correctly
