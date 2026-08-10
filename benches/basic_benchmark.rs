// Basic benchmark setup for bead-forge
//
// Run with: cargo bench
//
// This file provides a minimal benchmarking setup using Criterion.
// Add specific benchmarks for performance-critical operations.

use bead_forge::{Issue, IssueType, Priority, Status};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tempfile::TempDir;
use std::fs;

fn bench_bead_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("bead_creation");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("create_beads", size), size, |b, &size| {
            b.iter(|| {
                let temp_dir = TempDir::new().unwrap();
                let beads_dir = temp_dir.path().join(".beads");
                fs::create_dir(&beads_dir).unwrap();

                bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();
                let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
                let db_path = beads_dir.join(&metadata.database);

                let storage = bead_forge::Storage::open(&db_path).unwrap();

                for i in 0..size {
                    let bead = Issue::new(
                        format!("bf-test-{:04}", i),
                        format!("Test bead {}", i),
                        ".".to_string()
                    );
                    storage.create_issue(&bead).unwrap();
                }

                black_box(temp_dir)
            })
        });
    }

    group.finish();
}

fn bench_bead_query(c: &mut Criterion) {
    c.bench_function("query_single_bead", |b| {
        let temp_dir = TempDir::new().unwrap();
        let beads_dir = temp_dir.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        bead_forge::config::init_workspace(&beads_dir, "bf").unwrap();
        let metadata = bead_forge::config::load_metadata(&beads_dir).unwrap();
        let db_path = beads_dir.join(&metadata.database);

        let storage = bead_forge::Storage::open(&db_path).unwrap();

        // Create 100 beads
        for i in 0..100 {
            let bead = Issue::new(
                format!("bf-test-{:04}", i),
                format!("Test bead {}", i),
                ".".to_string()
            );
            storage.create_issue(&bead).unwrap();
        }

        b.iter(|| {
            black_box(storage.get_issue("bf-test-0050").unwrap())
        });
    });
}

criterion_group!(
    benches,
    bench_bead_creation,
    bench_bead_query
);
criterion_main!(benches);
