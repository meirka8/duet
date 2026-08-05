use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
#[cfg(test)]
use tempfile::TempDir;

#[derive(Debug, Clone)]
pub struct CorpusOptions {
    pub seed: u64,
    pub total_entries: usize,
    pub deep_tree_depth: usize,
    pub unicode_percentage: usize,
    pub sparse_percentage: usize,
    pub hardlink_farm: bool,
    pub broken_symlinks: bool,
}

impl Default for CorpusOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            total_entries: 1000,
            deep_tree_depth: 10,
            unicode_percentage: 20,
            sparse_percentage: 5,
            hardlink_farm: true,
            broken_symlinks: true,
        }
    }
}

pub struct SimplePcg {
    state: u64,
    inc: u64,
}

impl SimplePcg {
    pub fn new(seed: u64) -> Self {
        let mut pcg = Self {
            state: seed.wrapping_add(0xDA3E39CB94B95BDB),
            inc: (seed << 1) | 1,
        };
        pcg.next_u64();
        pcg
    }

    pub fn next_u64(&mut self) -> u64 {
        let oldstate = self.state;
        self.state = oldstate.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((oldstate >> 18) ^ oldstate) >> 27) as u32;
        let rot = (oldstate >> 59) as u32;
        let val32 = (xorshifted >> rot) | (xorshifted << ((!rot).wrapping_add(1) & 31));
        ((val32 as u64) << 32) | (val32 as u64)
    }
}

pub fn generate_corpus(root: &Path, opts: &CorpusOptions) -> std::io::Result<usize> {
    fs::create_dir_all(root)?;
    let mut rng = SimplePcg::new(opts.seed);

    let unicode_names = [
        "中文测试",
        "русский_файл",
        "ملف_عربي",
        "🚀_rocket_file",
        "spaces in name",
        "📄_document_📊",
        "unicode_αβγδ",
        "日本語ファイル",
        "한국어_파일",
    ];

    let mut created = 0;
    let mut dir_stack: Vec<PathBuf> = vec![root.to_path_buf()];

    let mut current_dir = root.to_path_buf();
    for d in 0..opts.deep_tree_depth {
        let name = format!("deep_dir_{d:02}");
        current_dir = current_dir.join(name);
        fs::create_dir_all(&current_dir)?;
        dir_stack.push(current_dir.clone());
        created += 1;
        if created >= opts.total_entries {
            break;
        }
    }

    let mut master_file: Option<PathBuf> = None;

    while created < opts.total_entries {
        let parent_idx = (rng.next_u64() as usize) % dir_stack.len();
        let parent = dir_stack[parent_idx].clone();

        let roll = rng.next_u64() % 100;
        let is_dir = roll < 15;
        let is_unicode = roll >= 15 && roll < (15 + opts.unicode_percentage as u64);
        let is_sparse = roll >= (15 + opts.unicode_percentage as u64)
            && roll < (15 + opts.unicode_percentage as u64 + opts.sparse_percentage as u64);
        let is_symlink = opts.broken_symlinks && roll > 90;
        let is_hardlink = opts.hardlink_farm && master_file.is_some() && roll > 80 && roll <= 90;

        let name = if is_unicode {
            let u_idx = (rng.next_u64() as usize) % unicode_names.len();
            format!("{}_{:06}.bin", unicode_names[u_idx], created)
        } else {
            format!("entry_{:06}.dat", created)
        };

        let target_path = parent.join(name);

        if is_dir {
            if fs::create_dir_all(&target_path).is_ok() {
                if dir_stack.len() < 500 {
                    dir_stack.push(target_path);
                }
            }
        } else if is_symlink {
            let _ = std::os::unix::fs::symlink(parent.join("broken_target.tmp"), &target_path);
        } else if is_hardlink {
            if let Some(ref m) = master_file {
                let _ = fs::hard_link(m, &target_path);
            }
        } else if is_sparse {
            if let Ok(mut f) = File::create(&target_path) {
                let _ = f.seek(SeekFrom::Start(1024 * 1024));
                let _ = f.write_all(b"SPARSE_FOOTER");
            }
        } else {
            let content_len = (rng.next_u64() % 512) as usize;
            let buf = vec![(created % 256) as u8; content_len];
            let _ = fs::write(&target_path, &buf);
            if master_file.is_none() {
                master_file = Some(target_path);
            }
        }

        created += 1;
    }

    Ok(created)
}

#[tokio::test]
async fn test_corpus_generation_10_entries() {
    let temp = TempDir::new().unwrap();
    let opts = CorpusOptions {
        total_entries: 10,
        deep_tree_depth: 2,
        ..Default::default()
    };
    let count = generate_corpus(temp.path(), &opts).unwrap();
    assert_eq!(count, 10);
}

#[tokio::test]
async fn test_corpus_generation_1k_entries() {
    let temp = TempDir::new().unwrap();
    let opts = CorpusOptions {
        total_entries: 1000,
        deep_tree_depth: 5,
        ..Default::default()
    };
    let count = generate_corpus(temp.path(), &opts).unwrap();
    assert_eq!(count, 1000);
}

#[tokio::test]
async fn test_corpus_generation_100k_entries_fast() {
    let temp = TempDir::new().unwrap();
    let opts = CorpusOptions {
        total_entries: 5000,
        deep_tree_depth: 10,
        ..Default::default()
    };
    let count = generate_corpus(temp.path(), &opts).unwrap();
    assert_eq!(count, 5000);
}
