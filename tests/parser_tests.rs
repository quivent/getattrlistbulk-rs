//! Parser tests with synthetic buffer data.
//!
//! These tests construct known byte sequences to verify the buffer
//! parser handles various scenarios correctly.

use getattrlistbulk::{read_dir, DirReader, RequestedAttributes, ObjectType};
use std::fs;
use tempfile::tempdir;

/// Test parsing entries with only name attribute
#[test]
fn test_parse_name_only() {
    let dir = tempdir().expect("create temp dir");
    fs::write(dir.path().join("simple.txt"), "content").expect("write file");

    let attrs = RequestedAttributes {
        name: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "simple.txt");
    // Other fields should be None when not requested
    assert!(entries[0].size.is_none());
    assert!(entries[0].object_type.is_none());
}

/// Test parsing entries with all attributes
#[test]
fn test_parse_all_attributes() {
    let dir = tempdir().expect("create temp dir");
    let content = "test content 12345";
    fs::write(dir.path().join("full.txt"), content).expect("write file");

    let attrs = RequestedAttributes::all();

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    assert_eq!(entry.name, "full.txt");
    assert_eq!(entry.object_type, Some(ObjectType::Regular));
    assert_eq!(entry.size, Some(content.len() as u64));
    assert!(entry.alloc_size.is_some());
    assert!(entry.modified_time.is_some());
    assert!(entry.permissions.is_some());
    assert!(entry.inode.is_some());
    // entry_count is only for directories
    assert!(entry.entry_count.is_none());
}

/// Test parsing directory entries (vs files)
#[test]
fn test_parse_directory_entry() {
    let dir = tempdir().expect("create temp dir");
    fs::create_dir(dir.path().join("subdir")).expect("create subdir");
    fs::write(dir.path().join("subdir").join("child.txt"), "x").expect("write child");

    let attrs = RequestedAttributes {
        name: true,
        object_type: true,
        entry_count: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    assert_eq!(entry.name, "subdir");
    assert_eq!(entry.object_type, Some(ObjectType::Directory));
    assert!(entry.is_dir());
    // entry_count should be present for directories
    assert!(entry.entry_count.is_some());
    assert_eq!(entry.entry_count, Some(1)); // contains child.txt
}

/// Test parsing multiple entries
#[test]
fn test_parse_multiple_entries() {
    let dir = tempdir().expect("create temp dir");

    for i in 0..10 {
        fs::write(dir.path().join(format!("file_{}.txt", i)), format!("content {}", i))
            .expect("write file");
    }

    let attrs = RequestedAttributes {
        name: true,
        size: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 10);

    // Verify all names are present (order may vary)
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    for i in 0..10 {
        let expected = format!("file_{}.txt", i);
        assert!(names.contains(&expected.as_str()), "missing {}", expected);
    }
}

/// Test parsing entries with various filename characters
#[test]
fn test_parse_special_filenames() {
    let dir = tempdir().expect("create temp dir");

    let filenames = [
        "normal.txt",
        "with spaces.txt",
        "with-dashes.txt",
        "with_underscores.txt",
        "UPPERCASE.TXT",
        "MixedCase.Txt",
        "123numeric.txt",
        ".hidden",
    ];

    for name in &filenames {
        fs::write(dir.path().join(name), "x").expect("write file");
    }

    let attrs = RequestedAttributes {
        name: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), filenames.len());

    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    for expected in &filenames {
        assert!(names.contains(expected), "missing {}", expected);
    }
}

/// Test parsing with different buffer sizes
#[test]
fn test_parse_with_small_buffer() {
    let dir = tempdir().expect("create temp dir");

    // Create enough files to require multiple buffer fills with small buffer
    for i in 0..50 {
        fs::write(dir.path().join(format!("file_{:03}.txt", i)), format!("content {}", i))
            .expect("write file");
    }

    let attrs = RequestedAttributes {
        name: true,
        size: true,
        ..Default::default()
    };

    // Use very small buffer to force multiple syscalls
    let entries: Vec<_> = getattrlistbulk::read_dir_with_buffer(dir.path(), attrs, 4 * 1024)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 50, "should read all files even with small buffer");
}

/// Test ObjectType parsing for various file types
#[test]
fn test_object_type_parsing() {
    let dir = tempdir().expect("create temp dir");

    // Create a regular file
    fs::write(dir.path().join("regular.txt"), "content").expect("write file");

    // Create a directory
    fs::create_dir(dir.path().join("directory")).expect("create dir");

    // Create a symlink (if supported)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink("regular.txt", dir.path().join("link.txt"));
    }

    let attrs = RequestedAttributes {
        name: true,
        object_type: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    // Find and verify each type
    let regular = entries.iter().find(|e| e.name == "regular.txt");
    assert!(regular.is_some());
    assert_eq!(regular.unwrap().object_type, Some(ObjectType::Regular));
    assert!(regular.unwrap().is_file());

    let directory = entries.iter().find(|e| e.name == "directory");
    assert!(directory.is_some());
    assert_eq!(directory.unwrap().object_type, Some(ObjectType::Directory));
    assert!(directory.unwrap().is_dir());

    #[cfg(unix)]
    {
        if let Some(link) = entries.iter().find(|e| e.name == "link.txt") {
            // Symlinks may or may not be followed depending on options
            assert!(link.object_type.is_some());
        }
    }
}

/// Test DirEntry helper methods
#[test]
fn test_dir_entry_helpers() {
    let dir = tempdir().expect("create temp dir");
    fs::write(dir.path().join("file.txt"), "x").expect("write file");
    fs::create_dir(dir.path().join("subdir")).expect("create dir");

    let attrs = RequestedAttributes {
        name: true,
        object_type: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    let file = entries.iter().find(|e| e.name == "file.txt").unwrap();
    let subdir = entries.iter().find(|e| e.name == "subdir").unwrap();

    assert!(file.is_file());
    assert!(!file.is_dir());
    assert!(!file.is_symlink());

    assert!(subdir.is_dir());
    assert!(!subdir.is_file());
    assert!(!subdir.is_symlink());
}

/// Test builder with selective attributes
#[test]
fn test_builder_selective_attributes() {
    let dir = tempdir().expect("create temp dir");
    fs::write(dir.path().join("test.txt"), "hello world").expect("write file");

    // Request only name and size
    let entries: Vec<_> = DirReader::new(dir.path())
        .name()
        .size()
        .read()
        .expect("read dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];

    assert_eq!(entry.name, "test.txt");
    assert_eq!(entry.size, Some(11)); // "hello world" = 11 bytes

    // object_type was not requested
    assert!(entry.object_type.is_none());
}
