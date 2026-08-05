use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use duet_types::VPath;
use duet_vfs::{FileSystem, ListOpts, local::LocalFs};
use futures::StreamExt;
use tempfile::TempDir;

fn bench_dir_enumeration(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let sizes = [10, 100, 1000];

    for &size in &sizes {
        c.bench_with_input(
            BenchmarkId::new("LocalFs::read_dir", size),
            &size,
            |b, &size| {
                // Generate a temporary directory with files
                let temp_dir = TempDir::new().unwrap();
                for i in 0..size {
                    let file_path = temp_dir.path().join(format!("file_{}.txt", i));
                    std::fs::write(file_path, "hello").unwrap();
                }

                let path_str = temp_dir.path().to_str().unwrap().to_string();
                let vpath = VPath::new_local(&path_str);
                let fs = LocalFs::new();

                b.to_async(&rt).iter(|| async {
                    let mut stream = fs.read_dir(&vpath, ListOpts::default());
                    let mut count = 0;
                    while let Some(chunk) = stream.next().await {
                        let entries = chunk.unwrap();
                        count += entries.len();
                    }
                    assert_eq!(count, size as usize);
                });
            },
        );
    }
}

criterion_group!(benches, bench_dir_enumeration);
criterion_main!(benches);
