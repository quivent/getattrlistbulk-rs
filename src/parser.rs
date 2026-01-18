//! Buffer parsing logic for getattrlistbulk results.
//!
//! This module handles parsing the raw byte buffer returned by getattrlistbulk
//! into structured DirEntry values.
//!
//! # Buffer Format
//!
//! Each entry in the buffer has the format:
//! ```text
//! +------------------+
//! | length (u32)     |  Total length of this entry
//! +------------------+
//! | attribute_set    |  Which attributes are present (20 bytes)
//! +------------------+
//! | fixed attrs      |  Fixed-size attributes in order
//! +------------------+
//! | attrreference    |  For names: offset (i32) + length (u32)
//! +------------------+
//! | variable data    |  Variable-length data at end
//! +------------------+
//! ```

use crate::error::ParseError;
use crate::ffi;
use crate::types::{DirEntry, ObjectType, RequestedAttributes};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Parser for getattrlistbulk result buffer.
pub(crate) struct BufferParser<'a> {
    buffer: &'a [u8],
    offset: usize,
    bytes_valid: usize,
    _requested: RequestedAttributes,
}

impl<'a> BufferParser<'a> {
    /// Create a new parser for the given buffer.
    pub fn new(buffer: &'a [u8], bytes_valid: usize, requested: RequestedAttributes) -> Self {
        Self {
            buffer,
            offset: 0,
            bytes_valid,
            _requested: requested,
        }
    }

    /// Parse the next entry from the buffer.
    ///
    /// Returns `None` when the buffer is exhausted.
    /// Returns `Some(Err(...))` on parse errors.
    pub fn next_entry(&mut self) -> Option<Result<DirEntry, ParseError>> {
        if self.offset >= self.bytes_valid {
            return None;
        }

        // Read entry length
        let entry_length = match self.read_u32(self.offset) {
            Ok(len) => len as usize,
            Err(e) => return Some(Err(e)),
        };

        if entry_length == 0 {
            return Some(Err(ParseError::InvalidEntryLength));
        }

        if self.offset + entry_length > self.bytes_valid {
            return Some(Err(ParseError::BufferTooSmall));
        }

        let result = self.parse_entry(self.offset, entry_length);
        self.offset += entry_length;
        Some(result)
    }

    /// Reset parser for a new buffer.
    #[allow(dead_code)]
    pub fn reset(&mut self, buffer: &'a [u8], bytes_valid: usize) {
        self.buffer = buffer;
        self.bytes_valid = bytes_valid;
        self.offset = 0;
    }

    fn parse_entry(&self, entry_start: usize, _entry_length: usize) -> Result<DirEntry, ParseError> {
        let mut offset = entry_start + 4; // Skip length field

        // Read returned attributes bitmap
        let returned = self.read_attribute_set(offset)?;
        offset += std::mem::size_of::<ffi::attribute_set>();

        // Parse attributes in order based on what was returned
        let mut name = String::new();
        let mut object_type = None;
        let mut size = None;
        let mut alloc_size = None;
        let mut modified_time = None;
        let mut permissions = None;
        let mut inode = None;
        let mut entry_count = None;

        // Common attributes (in order defined by macOS)
        if returned.commonattr & ffi::CommonAttr::NAME.bits() != 0 {
            let (parsed_name, new_offset) = self.parse_attrreference_string(entry_start, offset)?;
            name = parsed_name;
            offset = new_offset;
        }

        if returned.commonattr & ffi::CommonAttr::OBJTYPE.bits() != 0 {
            let vtype = self.read_u32(offset)?;
            object_type = Some(ObjectType::from(vtype));
            offset += 4;
        }

        if returned.commonattr & ffi::CommonAttr::MODTIME.bits() != 0 {
            let (time, new_offset) = self.parse_timespec(offset)?;
            modified_time = Some(time);
            offset = new_offset;
        }

        if returned.commonattr & ffi::CommonAttr::ACCESSMASK.bits() != 0 {
            permissions = Some(self.read_u32(offset)?);
            offset += 4;
        }

        if returned.commonattr & ffi::CommonAttr::FILEID.bits() != 0 {
            inode = Some(self.read_u64(offset)?);
            offset += 8;
        }

        // File attributes
        if returned.fileattr & ffi::FileAttr::TOTALSIZE.bits() != 0 {
            size = Some(self.read_u64(offset)?);
            offset += 8;
        }

        if returned.fileattr & ffi::FileAttr::ALLOCSIZE.bits() != 0 {
            alloc_size = Some(self.read_u64(offset)?);
            offset += 8;
        }

        // Directory attributes
        if returned.dirattr & ffi::DirAttr::ENTRYCOUNT.bits() != 0 {
            entry_count = Some(self.read_u32(offset)?);
            // offset += 4; // Not needed, we're done
        }

        Ok(DirEntry {
            name,
            object_type,
            size,
            alloc_size,
            modified_time,
            permissions,
            inode,
            entry_count,
        })
    }

    fn read_u32(&self, offset: usize) -> Result<u32, ParseError> {
        if offset + 4 > self.buffer.len() {
            return Err(ParseError::UnexpectedEnd);
        }
        let bytes: [u8; 4] = self.buffer[offset..offset + 4]
            .try_into()
            .map_err(|_| ParseError::UnexpectedEnd)?;
        Ok(u32::from_ne_bytes(bytes))
    }

    fn read_u64(&self, offset: usize) -> Result<u64, ParseError> {
        if offset + 8 > self.buffer.len() {
            return Err(ParseError::UnexpectedEnd);
        }
        let bytes: [u8; 8] = self.buffer[offset..offset + 8]
            .try_into()
            .map_err(|_| ParseError::UnexpectedEnd)?;
        Ok(u64::from_ne_bytes(bytes))
    }

    fn read_i32(&self, offset: usize) -> Result<i32, ParseError> {
        if offset + 4 > self.buffer.len() {
            return Err(ParseError::UnexpectedEnd);
        }
        let bytes: [u8; 4] = self.buffer[offset..offset + 4]
            .try_into()
            .map_err(|_| ParseError::UnexpectedEnd)?;
        Ok(i32::from_ne_bytes(bytes))
    }

    fn read_i64(&self, offset: usize) -> Result<i64, ParseError> {
        if offset + 8 > self.buffer.len() {
            return Err(ParseError::UnexpectedEnd);
        }
        let bytes: [u8; 8] = self.buffer[offset..offset + 8]
            .try_into()
            .map_err(|_| ParseError::UnexpectedEnd)?;
        Ok(i64::from_ne_bytes(bytes))
    }

    fn read_attribute_set(&self, offset: usize) -> Result<ffi::attribute_set, ParseError> {
        let size = std::mem::size_of::<ffi::attribute_set>();
        if offset + size > self.buffer.len() {
            return Err(ParseError::UnexpectedEnd);
        }

        Ok(ffi::attribute_set {
            commonattr: self.read_u32(offset)?,
            volattr: self.read_u32(offset + 4)?,
            dirattr: self.read_u32(offset + 8)?,
            fileattr: self.read_u32(offset + 12)?,
            forkattr: self.read_u32(offset + 16)?,
        })
    }

    fn parse_attrreference_string(
        &self,
        _entry_start: usize,
        ref_offset: usize,
    ) -> Result<(String, usize), ParseError> {
        // Read attrreference: offset (i32) + length (u32)
        let data_offset = self.read_i32(ref_offset)?;
        let data_length = self.read_u32(ref_offset + 4)?;

        // Offset is relative to the attrreference location
        let string_start = (ref_offset as i32 + data_offset) as usize;
        let string_end = string_start + data_length as usize;

        // Bounds check
        if string_end > self.buffer.len() {
            return Err(ParseError::InvalidOffset);
        }

        // Extract string (excluding null terminator if present)
        let mut name_bytes = &self.buffer[string_start..string_end];
        if let Some(null_pos) = name_bytes.iter().position(|&b| b == 0) {
            name_bytes = &name_bytes[..null_pos];
        }

        // Convert to UTF-8, using lossy conversion for invalid sequences
        let name = String::from_utf8_lossy(name_bytes).into_owned();

        // Return the offset after the attrreference (8 bytes)
        Ok((name, ref_offset + 8))
    }

    fn parse_timespec(&self, offset: usize) -> Result<(SystemTime, usize), ParseError> {
        // timespec is: tv_sec (i64) + tv_nsec (i64) on 64-bit
        let tv_sec = self.read_i64(offset)?;
        let tv_nsec = self.read_i64(offset + 8)?;

        let duration = Duration::new(tv_sec as u64, tv_nsec as u32);
        let time = UNIX_EPOCH + duration;

        Ok((time, offset + 16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u32() {
        let buffer = [0x01, 0x02, 0x03, 0x04];
        let parser = BufferParser::new(&buffer, buffer.len(), RequestedAttributes::default());
        let value = parser.read_u32(0).unwrap();
        assert_eq!(value, u32::from_ne_bytes([0x01, 0x02, 0x03, 0x04]));
    }

    #[test]
    fn test_read_u32_bounds() {
        let buffer = [0x01, 0x02];
        let parser = BufferParser::new(&buffer, buffer.len(), RequestedAttributes::default());
        assert!(parser.read_u32(0).is_err());
    }
}
