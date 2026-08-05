# ADR-0007: VFS Backend Strategy — Hand-Rolled POSIX `LocalFs` Paired with OpenDAL Bridge (OQ-5 Resolution)

- **Status:** Accepted (Resolves Open Question OQ-5)
- **Deciders:** Lead Architect, Core VFS Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §17 OQ-5](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L730) / `NFR-03` / `NFR-07`

---

## Context and Problem Statement

Open Question **OQ-5** asks:
> *Should Duet own its entire VFS protocol stack, or lean on Apache OpenDAL for breadth and accept its abstraction overhead?*

Duet requires two distinct capabilities from its Virtual Filesystem (VFS) abstraction in [`crates/duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs):

1. **Maximum Local Filesystem Throughput on Linux (`NFR-03`, `NFR-07`):** First paint of 100,000 local files in $\le 100\text{ ms}$; copy throughput $\ge 95\%$ of `cp` for large files and $\ge 80\%$ of `cp -a` for 100,000 small files.
2. **Broad Cloud & Remote Storage Support:** Support for S3, SFTP, WebDAV, Google Cloud Storage (GCS), Azure Blob Storage, and HDFS.

Using a single generic VFS library for both local and remote filesystems creates a fundamental conflict: generic VFS abstractions abstract away Linux-specific kernel syscalls required for maximum throughput, while writing hand-rolled remote protocol clients for dozens of cloud providers requires excessive engineering maintenance.

---

## Decision Drivers

- **Linux Kernel Syscall Optimization:** Direct access to `getdents64` raw buffer streaming, fine-grained `statx` masks, zero-copy `copy_file_range`, btrfs/zfs `reflink` (`FICLONE`), and atomic `renameat2`.
- **Remote Storage Breadth:** Instant support for S3, SFTP, WebDAV, Azure, GCS without writing custom protocol drivers from scratch.
- **Unified VFS Trait Interface:** Both local and cloud backends must implement Duet's unified `Vfs` trait in [`duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs).

---

## Considered Options

1. **Hybrid Architecture (Chosen):** Hand-rolled POSIX `LocalFs` utilizing direct `rustix`/`libc` Linux syscalls, paired with an `OpenDalFs` bridge adapter for all cloud and remote protocols.
2. **Pure OpenDAL Everywhere:** Use Apache OpenDAL for all VFS backends including local disk (`opendal::services::Fs`).
3. **Pure Hand-Rolled VFS Everywhere:** Write custom local and remote protocol drivers (S3, SFTP, WebDAV) entirely within `duet-vfs`.

---

## Decision Outcome

**Chosen Strategy:** **Hybrid VFS Architecture (Option 1).**

Resolves Open Question **OQ-5**.

```
                        duet-vfs Unified Vfs Trait
                                    │
         ┌──────────────────────────┴──────────────────────────┐
         ▼                                                     ▼
  Hand-Rolled LocalFs                                  OpenDalFs Bridge Adapter
(Linux rustix/libc Syscalls)                           (Apache OpenDAL Engine)
  ├── raw getdents64 buffer streaming                    ├── Amazon S3 / MinIO
  ├── statx fine-grained attribute masks                 ├── SFTP / SSH
  ├── copy_file_range zero-copy offload                  ├── WebDAV
  ├── FICLONE / FICLONERANGE reflinks                    ├── Google Cloud Storage
  └── renameat2 RENAME_NOREPLACE                         └── Azure Blob / HDFS
```

### 1. Hand-Rolled POSIX `LocalFs`

For local filesystems on Linux, `duet-vfs` implements a dedicated `LocalFs` backend utilizing `rustix` and `libc` directly:
- **`getdents64` Streaming:** Bypasses legacy C-library `readdir()` allocation loops by streaming $64\text{ KB}$ raw kernel directory buffer blocks directly into Duet's Struct-of-Arrays interning arena ([ADR-0005](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0005-soa-directory-memory-layout.md)).
- **`statx` Masking:** Queries only requested attributes (`STATX_BASIC_STATS`), avoiding expensive xattr or ACL kernel lookups during fast directory scrolling.
- **Zero-Copy File Copying:** Uses the kernel copy ladder: `reflink` (`FICLONE`) $\to$ `copy_file_range` $\to$ sparse-buffered copy.

### 2. `OpenDalFs` Bridge Adapter

For remote and cloud filesystems, `duet-vfs` implements `OpenDalFs`, which wraps an `opendal::Operator`. The adapter translates Duet `Vfs` trait calls into OpenDAL async operations, granting instant access to 30+ storage backends.

---

## Pros and Cons of the Options

### Hybrid VFS Architecture (Chosen)

- **Good:** Achieves maximum local filesystem performance on Linux (`NFR-03`, `NFR-07`); instant out-of-the-box integration with S3, SFTP, WebDAV, and cloud storage via OpenDAL; zero compromise between local speed and cloud breadth.
- **Bad:** Requires maintaining two internal backend integration code paths inside `duet-vfs`.

### Pure OpenDAL Everywhere

- **Good:** Single uniform backend codebase; reduced internal VFS maintenance.
- **Bad:** OpenDAL's generic `Fs` service obscures `getdents64` streaming, `statx` masking, and `reflink` ioctls; fails `NFR-03` ($100\text{k}$ first paint $\le 100\text{ ms}$) and `NFR-07` copy throughput benchmarks.

### Pure Hand-Rolled VFS Everywhere

- **Good:** Complete control over every network socket and protocol byte.
- **Bad:** Massive engineering burden; requires building and maintaining custom S3, SFTP, WebDAV, GCS, and Azure client libraries.

---

## Consequences

### Positive

- Satisfies both `NFR-03` (sub-$100\text{ ms}$ local listing first paint) and `NFR-07` (native `cp` throughput parity).
- Delivers instant enterprise cloud connectivity (S3, SFTP, WebDAV) with minimal maintenance footprint.
- Clean separation of concerns within [`crates/duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs).

### Negative / Risks

- Feature capabilities (e.g. symlink handling or file permissions) differ between POSIX `LocalFs` and cloud `OpenDalFs` backends (addressed via `VfsCapability` flag masks in `duet-types`).

---

## Implementation & Architecture Details

- **Local POSIX Backend:** [`crates/duet-vfs/src/local.rs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs)
- **OpenDAL Bridge:** [`crates/duet-vfs/src/opendal_bridge.rs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs)
- **Unified Trait:** `Vfs` trait in [`crates/duet-types/src/vfs.rs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types).

---

## Validation Strategy

- **`NFR-03` Directory Listing Benchmark:** `benches/dir_enumeration.rs` comparing `LocalFs` `getdents64` streaming against standard `std::fs::read_dir`.
- **`NFR-07` Throughput Benchmark:** Criterion throughput suite comparing `LocalFs` `copy_file_range` against GNU `cp` coreutils.
- **Cloud Conformance Tests:** Automated integration test suite running `OpenDalFs` against local MinIO (S3) and test SFTP containers.
