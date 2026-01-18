//! Integration tests for getattrlistbulk crate.

use getattrlistbulk::{read_dir, DirReader, RequestedAttributes, Error};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_read_tmp_directory() {
    // /tmp always exists and has entries
    let attrs = RequestedAttributes {
        name: true,
        ..Default::default()
    };

    let entries: Vec<_> = read_dir("/tmp", attrs)
        .expect("should open /tmp")
        .filter_map(|e| e.ok())
        .collect();

    // /tmp should have at least something
    assert!(!entries.is_empty(), "/tmp should not be empty");
}

#[test]
fn test_read_with_all_attributes() {
    let dir = tempdir().expect("create temp dir");
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "hello world").expect("write file");

    let attrs = RequestedAttributes::all();
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("should open temp dir")
        .filter_map(|e| e.ok())
        .collect();

    let entry = entries.iter().find(|e| e.name == "test.txt")
        .expect("should find test.txt");

    assert!(entry.size.is_some(), "size should be present");
    assert_eq!(entry.size.unwrap(), 11, "size should be 11 bytes");
    assert!(entry.object_type.is_some(), "object_type should be present");
}

#[test]
fn test_metadata_matches_std_fs() {
    let dir = tempdir().expect("create temp dir");
    let file_path = dir.path().join("compare.txt");
    fs::write(&file_path, "test content for comparison").expect("write file");

    let attrs = RequestedAttributes::all();
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("open dir")
        .filter_map(|e| e.ok())
        .collect();

    let entry = entries.iter().find(|e| e.name == "compare.txt").unwrap();
    let std_meta = fs::metadata(&file_path).expect("std metadata");

    assert_eq!(entry.size.unwrap(), std_meta.len());
}

#[test]
fn test_empty_directory() {
    let dir = tempdir().expect("create temp dir");

    let attrs = RequestedAttributes { name: true, ..Default::default() };
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("open empty dir")
        .collect();

    assert!(entries.is_empty(), "empty dir should have no entries");
}

#[test]
fn test_nonexistent_directory() {
    let attrs = RequestedAttributes::default();
    let result = read_dir("/nonexistent/path/that/does/not/exist", attrs);

    assert!(result.is_err(), "should error on nonexistent path");
    match result {
        Err(Error::Open(_)) => {} // expected
        Err(e) => panic!("expected Error::Open, got {:?}", e),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[test]
fn test_permission_denied() {
    // /private/var/root requires root access
    let attrs = RequestedAttributes::default();
    let result = read_dir("/private/var/root", attrs);

    // Should either error on open or during iteration
    assert!(result.is_err() || result.unwrap().any(|e| e.is_err()));
}

#[test]
fn test_builder_api() {
    let dir = tempdir().expect("create temp dir");
    fs::write(dir.path().join("builder_test.txt"), "test").expect("write");

    let entries: Vec<_> = DirReader::new(dir.path())
        .name()
        .size()
        .object_type()
        .buffer_size(32 * 1024)
        .read()
        .expect("read via builder")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "builder_test.txt");
}

#[test]
fn test_unicode_filenames() {
    let dir = tempdir().expect("create temp dir");
    let unicode_name = "日本語ファイル.txt";
    fs::write(dir.path().join(unicode_name), "content").expect("write unicode file");

    let attrs = RequestedAttributes { name: true, ..Default::default() };
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("open dir")
        .filter_map(|e| e.ok())
        .collect();

    assert!(entries.iter().any(|e| e.name == unicode_name), "should handle unicode");
}

#[test]
fn test_many_files() {
    let dir = tempdir().expect("create temp dir");

    // Create 100 files
    for i in 0..100 {
        fs::write(dir.path().join(format!("file_{:03}.txt", i)), format!("content {}", i))
            .expect("write file");
    }

    let attrs = RequestedAttributes { name: true, size: true, ..Default::default() };
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("open dir")
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 100, "should read all 100 files");
}

#[test]
fn test_subdirectories() {
    let dir = tempdir().expect("create temp dir");
    fs::create_dir(dir.path().join("subdir")).expect("create subdir");
    fs::write(dir.path().join("file.txt"), "content").expect("write file");

    let attrs = RequestedAttributes { name: true, object_type: true, ..Default::default() };
    let entries: Vec<_> = read_dir(dir.path(), attrs)
        .expect("open dir")
        .filter_map(|e| e.ok())
        .collect();

    let subdir = entries.iter().find(|e| e.name == "subdir").expect("find subdir");
    let file = entries.iter().find(|e| e.name == "file.txt").expect("find file");

    assert!(subdir.is_dir());
    assert!(file.is_file());
}
