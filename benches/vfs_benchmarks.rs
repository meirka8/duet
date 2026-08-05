use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use duet_index::{DirectoryModel, EntryInput, FilterSpec, SortColumn, SortDirection};
use duet_types::{EntryId, FileType, VPath};
use duet_vfs::{local::LocalFs, FileSystem, ListOpts};
use futures::StreamExt;
use std::time::Duration;
use tempfile::TempDir;

#[path = "../tests/corpus_generator.rs"]
mod corpus_generator;
use corpus_generator::{generate_corpus, CorpusOptions};

fn bench_vfs_read_dir(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("VFS read_dir");
    group.measurement_time(Duration::from_secs(3));

    for &size in &[10, 1000, 10000] {
        let temp = TempDir::new().unwrap();
        let opts = CorpusOptions {
            total_entries: size,
            deep_tree_depth: 3,
            ..Default::default()
        };
        let _ = generate_corpus(temp.path(), &opts);

        let path_str = temp.path().to_str().unwrap().to_string();
        let vpath = VPath::new_local(&path_str);
        let fs = LocalFs::new();

        group.bench_with_input(BenchmarkId::new("read_dir", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let mut stream = fs.read_dir(&vpath, ListOpts::default());
                let mut count = 0;
                while let Some(chunk) = stream.next().await {
                    let entries = chunk.unwrap();
                    count += entries.len();
                }
                count
            });
        });
    }
    group.finish();
}

fn bench_statx_batching(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("VFS statx batching");
    group.measurement_time(Duration::from_secs(3));

    for &size in &[10, 1000, 10000] {
        let temp = TempDir::new().unwrap();
        let opts = CorpusOptions {
            total_entries: size,
            deep_tree_depth: 3,
            ..Default::default()
        };
        let _ = generate_corpus(temp.path(), &opts);

        let path_str = temp.path().to_str().unwrap().to_string();
        let vpath = VPath::new_local(&path_str);
        let fs = LocalFs::new();
        let list_opts = ListOpts {
            size: true,
            mtime: true,
            mode: true,
            file_type: true,
        };

        group.bench_with_input(BenchmarkId::new("statx_batch", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let mut stream = fs.read_dir(&vpath, list_opts);
                let mut stat_count = 0;
                while let Some(chunk) = stream.next().await {
                    let entries = chunk.unwrap();
                    for entry in entries {
                        if entry.metadata.is_some() {
                            stat_count += 1;
                        }
                    }
                }
                stat_count
            });
        });
    }
    group.finish();
}

fn bench_entry_store_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("EntryStore sorting");
    group.measurement_time(Duration::from_secs(3));

    for &size in &[10, 1000, 100000] {
        let entries: Vec<EntryInput> = (0..size)
            .map(|i| EntryInput {
                id: EntryId(i as u64),
                name: format!("entry_{:06}_{}.txt", size - i, i % 10),
                file_type: if i % 5 == 0 {
                    FileType::Directory
                } else {
                    FileType::File
                },
                size: (i * 1024) as u64,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1700000000 + (i as i64),
                atime: 1700000000 + (i as i64),
                ctime: 1700000000 + (i as i64),
                dev: 1,
                ino: i as u64,
                nlink: 1,
                flags: 0,
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("sort_name", size), &size, |b, _| {
            b.iter_batched(
                || {
                    let mut model = DirectoryModel::new();
                    model.set_entries(entries.clone());
                    model
                },
                |mut model| {
                    model.sort(SortColumn::Name, SortDirection::Ascending);
                },
                criterion::BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("sort_size", size), &size, |b, _| {
            b.iter_batched(
                || {
                    let mut model = DirectoryModel::new();
                    model.set_entries(entries.clone());
                    model
                },
                |mut model| {
                    model.sort(SortColumn::Size, SortDirection::Descending);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_quick_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("DirectoryModel quick-filtering");
    group.measurement_time(Duration::from_secs(3));

    for &size in &[10, 1000, 100000] {
        let entries: Vec<EntryInput> = (0..size)
            .map(|i| EntryInput {
                id: EntryId(i as u64),
                name: if i % 2 == 0 {
                    format!("match_pattern_{i:06}.dat")
                } else {
                    format!("other_file_{i:06}.bin")
                },
                file_type: if i % 10 == 0 {
                    FileType::Directory
                } else {
                    FileType::File
                },
                size: (i * 512) as u64,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1700000000 + (i as i64),
                atime: 1700000000 + (i as i64),
                ctime: 1700000000 + (i as i64),
                dev: 1,
                ino: i as u64,
                nlink: 1,
                flags: 0,
            })
            .collect();

        let filter = FilterSpec {
            show_hidden: true,
            quick_filter: Some("match_pattern".into()),
            mask: None,
        };

        group.bench_with_input(BenchmarkId::new("quick_filter", size), &size, |b, _| {
            b.iter_batched(
                || {
                    let mut model = DirectoryModel::new();
                    model.set_entries(entries.clone());
                    model
                },
                |mut model| {
                    model.filter(filter.clone());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_vfs_read_dir,
    bench_statx_batching,
    bench_entry_store_sorting,
    bench_quick_filtering
);
criterion_main!(benches);
