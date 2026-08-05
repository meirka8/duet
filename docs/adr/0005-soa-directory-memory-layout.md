# ADR-0005: Struct-of-Arrays (SoA) Directory Model with String Interning

- **Status:** Accepted
- **Deciders:** Lead Architect, Core Systems Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §8.2](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L428) / `NFR-04` / `NFR-05` / `NFR-06`

---

## Context and Problem Statement

Duet aims to display directory listings containing up to **1,000,000 file entries** within a single dual-pane file view while remaining fluid ($120\text{ Hz}$ scroll refresh rate, zero UI stalls $> 16\text{ ms}$) and memory-efficient ($\le 150\text{ MB}$ RSS total overhead, `NFR-06`).

Standard Rust object-oriented layout represents directory entries as an **Array-of-Structs (AoS)**:

```rust
// Traditional Array-of-Structs (AoS) - REJECTED
pub struct FileEntry {
    pub name: String,         // 24B struct + heap allocation (~32B avg)
    pub size: u64,            // 8B
    pub mtime: i64,           // 8B
    pub atime: i64,           // 8B
    pub mode: u32,            // 4B
    pub flags: u32,           // 4B
    pub inode: u64,           // 8B
    pub meta_index: u32,      // 4B
}
// Vec<FileEntry> consumes 160B-256B per entry with heap fragmentation.
```

Under AoS, $1,000,000$ entries consume **$200\text{ MB}$ to $300\text{ MB}$ of RAM per pane**, causing heavy heap fragmentation, pointer-chasing during sorting, and CPU cache misses that make sub-$16\text{ ms}$ sorting and zero-allocation $120\text{ Hz}$ scrolling mathematically impossible.

---

## Decision Drivers

- **Memory Budget (`NFR-06`):** Target $\le 96\text{ bytes}$ average total memory budget per file entry ($1\text{M}$ entries fit in $\sim 96\text{ MB}$).
- **Directory Load Time (`NFR-04`):** Fully load, parse, and sort $1,000,000$ entries in $\le 3\text{ s}$.
- **Scrolling & Render Performance (`NFR-05`):** $120\text{ Hz}$ table scrolling without per-frame memory allocations or CPU cache misses.
- **Fast Sorting & Filtering:** Enable SIMD-accelerated array scans and sub-millisecond column sorting.

---

## Decision Outcome

**Chosen Model:** **Struct-of-Arrays (SoA)** directory memory layout backed by a **Contiguous String Interning Arena**.

Implemented in [`crates/duet-types`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types) and [`crates/duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs).

```
                      Struct-of-Arrays (SoA) Memory Layout
                     ┌────────────────────────────────────┐
  order: Vec<u32>    │ 0 │ 3 │ 1 │ 2 │ 4 │ ...            │ (4B per entry)
                     └─┬───┬───┬──────────────────────────┘
                       │   │   │
  names: Vec<StringId> ├───┼───┼──> String Arena Pool ("Box<str> Slabs")
  sizes: Vec<u64>      ├───┼───┼──> [1024, 4096, 512, 98304, ...]  (8B)
  mtime: Vec<i64>      ├───┼───┼──> [1722860000, 1722861000, ...]  (8B)
  atime: Vec<i64>      ├───┼───┼──> [1722860500, 1722861500, ...]  (8B)
  mode:  Vec<u32>      ├───┼───┼──> [0o755, 0o644, 0o600, ...]     (4B)
  flags: Vec<u32>      └───┴───┴──> [DIR|EXEC, REG, HIDDEN, ...]   (4B)
```

### Memory Budget Breakdown (Target: $\le 96\text{ B}$ / entry)

| Component | Layout Primitive | Size per Entry |
|---|---|---|
| **Filename Reference** | `names: Vec<StringId>` (32-bit handle) | $4\text{ bytes}$ |
| **File Size** | `sizes: Vec<u64>` | $8\text{ bytes}$ |
| **Modification Time** | `mtime: Vec<i64>` | $8\text{ bytes}$ |
| **Access Time** | `atime: Vec<i64>` | $8\text{ bytes}$ |
| **Permissions / Mode** | `mode: Vec<u32>` | $4\text{ bytes}$ |
| **Attributes & Flags** | `flags: Vec<u32>` (dir, symlink, hidden, exec, selected) | $4\text{ bytes}$ |
| **Inode ID** | `inode: Vec<u64>` | $8\text{ bytes}$ |
| **Metadata Index** | `meta_idx: Vec<u32>` | $4\text{ bytes}$ |
| **Sort Permutation Index** | `order: Vec<u32>` | $4\text{ bytes}$ |
| **Fixed Array Total** | **Contiguous Primitive Vectors** | **$52\text{ bytes}$** |
| **String Arena Pool** | Packed contiguous `Box<str>` slabs (~$30\text{B}$ avg filename + arena overhead) | **$\sim 44\text{ bytes}$** |
| **TOTAL BUDGET** | **Combined SoA + String Pool** | **$\mathbf{\approx 96\text{ bytes / entry}}$** |

---

## Technical Advantages

1. **Zero Heap Allocation Sorting:** Sorting directory listings by size, modification time, or extension permutes **only the 4-byte `order: Vec<u32>` index vector**. The heavy underlying attribute arrays and string buffers are never swapped or moved.
2. **CPU Cache Saturation:** When sorting by file size, the CPU cache line ($64\text{ bytes}$) loads **8 full `u64` size entries simultaneously**, maximizing L1/L2 cache hit rates and enabling auto-vectorized SIMD comparisons.
3. **Zero Per-Frame Rendering Allocations:** GPUI virtualized table delegates query row attributes directly from slice indexes (`sizes[order[row]]`), eliminating string clones or heap allocations during $120\text{ Hz}$ scrolling.

---

## Pros and Cons of the Options

### Struct-of-Arrays (SoA) + String Interning (Chosen)

- **Good:** Consumes $\sim 96\text{ MB}$ total RAM for $1,000,000$ entries (fits inside $\le 150\text{ MB}$ RSS budget); sub-second sorting of $1\text{M}$ rows; optimal CPU L1/L2 cache line utilization; zero per-frame UI allocations.
- **Bad:** Slightly more complex indexing logic (`table.sizes[table.order[i]]` instead of `table[i].size`); string arena requires manual lifetime/compaction management when removing items.

### Array-of-Structs (AoS) (`Vec<FileEntry>`)

- **Good:** Idiomatic Rust code readability; simple object-oriented accessors.
- **Bad:** Consumes $> 250\text{ MB}$ RAM for $1\text{M}$ entries; high heap fragmentation; pointer chasing during sort; violates `NFR-04`, `NFR-05`, and `NFR-06`.

---

## Consequences

### Positive

- Achieves $1,000,000$ entry scale within strict $\le 150\text{ MB}$ RSS limit (`NFR-06`).
- Enables lightning-fast table sorting and filtering without UI thread blocking.
- Provides flat, zero-copy data slices directly consumable by GPUI table delegates.

### Negative / Risks

- Internal VFS code must maintain parallel vector length invariants (enforced by `DirectoryTable` builder abstractions in `duet-types`).

---

## Implementation & Architecture Details

- **Core Data Structure:** `DirectoryTable` in [`crates/duet-types/src/directory.rs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types).
- **String Interner:** `StringArena` in [`crates/duet-types/src/arena.rs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types).

---

## Validation Strategy

- **Memory Overhead Benchmark:** Criterion benchmark in `benches/dir_enumeration.rs` measuring RSS memory usage across $1,000$, $100,000$, and $1,000,000$ entries, asserting average byte footprint $\le 96\text{ B}$ per entry.
- **Sort Latency Benchmark:** Automated test asserting $1,000,000$ entry sort completes in $\le 400\text{ ms}$.
