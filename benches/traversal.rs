//! Benchmarks comparing getattrlistbulk vs std::fs.
//!
//! Run with: cargo bench

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use getattrlistbulk::{read_dir, RequestedAttributes};
use std::fs;

/// Benchmark getattrlistbulk on a real directory
fn bench_getattrlistbulk(c: &mut Criterion) {
    let mut group = c.benchmark_group("directory_read");

    // Test directories of different sizes
    let test_dirs = [
        ("/usr/bin", "usr_bin"),
        ("/usr/lib", "usr_lib"),
        ("/tmp", "tmp"),
    ];

    for (path, name) in test_dirs.iter() {
        if std::path::Path::new(path).exists() {
            group.bench_with_input(
                BenchmarkId::new("getattrlistbulk", name),
                path,
                |b, path| {
                    b.iter(|| {
                        let attrs = RequestedAttributes {
                            name: true,
                            size: true,
                            object_type: true,
                            ..Default::default()
                        };
                        let count: usize = read_dir(path, attrs)
                            .unwrap()
                            .filter_map(|e| e.ok())
                            .count();
                        count
                    })
                },
            );

            group.bench_with_input(
                BenchmarkId::new("std_fs", name),
                path,
                |b, path| {
                    b.iter(|| {
                        let count: usize = fs::read_dir(path)
                            .unwrap()
                            .filter_map(|e| e.ok())
                            .map(|e| {
                                let _ = e.metadata();
                                1
                            })
                            .count();
                        count
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmark with different buffer sizes
fn bench_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_size");

    let test_path = "/usr/lib";
    if !std::path::Path::new(test_path).exists() {
        return;
    }

    let buffer_sizes = [16 * 1024, 64 * 1024, 256 * 1024, 1024 * 1024];

    for size in buffer_sizes.iter() {
        group.bench_with_input(
            BenchmarkId::new("getattrlistbulk", format!("{}KB", size / 1024)),
            size,
            |b, &size| {
                b.iter(|| {
                    let attrs = RequestedAttributes {
                        name: true,
                        size: true,
                        ..Default::default()
                    };
                    let count: usize = getattrlistbulk::read_dir_with_buffer(test_path, attrs, size)
                        .unwrap()
                        .filter_map(|e| e.ok())
                        .count();
                    count
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_getattrlistbulk, bench_buffer_sizes);
criterion_main!(benches);
