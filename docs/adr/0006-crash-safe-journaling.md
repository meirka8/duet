# ADR-0006: Append-Only Write-Ahead Job Journaling and Partial Staging

- **Status:** Accepted
- **Deciders:** Lead Architect, Core Systems & Reliability Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §9.3](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L450) / [`docs/crash-safety.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/crash-safety.md) / `NFR-08` / `FR-OPS-07`

---

## Context and Problem Statement

File operations (copying, moving, deleting, truncating, metadata updates, and patching) are vulnerable to mid-operation failures caused by kernel panics, system power loss, process termination (`SIGKILL`), storage space exhaustion (`ENOSPC`), permission errors (`EACCES`), or network disconnects.

Naive file managers write directly to destination target paths. If interrupted mid-transfer, the destination file is left corrupt or partially written, while cross-device moves may unlink source files before destination durability is guaranteed.

To satisfy **`NFR-08` (Zero Data Loss/Corruption)**, Duet must guarantee deterministic transactional crash safety across all file operations.

---

## Decision Drivers

- **Core Safety Invariant (`NFR-08` / [`docs/crash-safety.md` §1](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/crash-safety.md#L7-L9)):**
  > *For every fault point during any file operation step, either the source is completely intact and the destination is absent or clearly marked partial (`.duet-partial-<rand>`), or the destination is complete, verified, and durable (`fsync`'d). Never a silently truncated or partially overwritten destination file.*
- **Zero Silent Data Loss:** Source files must never be unlinked or modified until destination durability is verified.
- **Deterministic Recovery:** Application startup must automatically detect, clean, or resume interrupted operations without requiring manual filesystem repairs.

---

## Decision Outcome

**Chosen Policy:** Mandatory Write-Ahead Job Journaling (`~/.local/state/duet/jobs/*.journal`) paired with Double-Buffered Partial Staging (`.duet-partial-<rand>`).

Implemented in [`crates/duet-ops`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ops).

```
                      Write-Ahead Journal State Machine
  ┌────────────┐    ┌────────────────────────┐    ┌─────────────────────┐
  │ JobStarted │───>│ StepStarted(step_id)   │───>│ StepProgress(offset)│
  └────────────┘    └───────────┬────────────┘    └──────────┬──────────┘
                                │                            │
    fsync() journal before      ▼                            ▼
    touching staging path    (SIGKILL / Power Loss Point) ───┼─> [Startup Recovery Scanner]
                                │                            │   - Unlinks .duet-partial-*
                                ▼                            │   - Resumes or prompts user
                    ┌────────────────────────┐               │
                    │ StepCompleted(step_id) │<──────────────┘
                    └───────────┬────────────┘
                                ▼
                         ┌──────────────┐
                         │ JobCompleted │
                         └──────────────┘
```

### Execution Lifecycle Protocol

1. **Write-Ahead Intent:** Before executing any destructive step, `StepStarted(step_id, src, dst, staged_dst)` is appended to `~/.local/state/duet/jobs/<job_id>.journal` and explicitly **`fsync()`ed to disk**.
2. **Partial Staging:** File payloads are written exclusively to a staged destination path in the target directory (`<dst_parent>/.duet-partial-<rand>`).
3. **Data Durability Flush:** Upon byte transfer completion, `fdatasync()` is issued on the staging file descriptor.
4. **Atomic Swap:** `renameat2(staged_dst, dst)` atomically commits the staged file to its final target path.
5. **Step Completion:** `StepCompleted(step_id)` is appended to the journal and flushed.

---

## Interruption Recovery Rules

When Duet starts up, the `duet-ops` recovery scanner inspects `~/.local/state/duet/jobs/*.journal`:

- **Interrupted `CopyFile`:** Scanner unlinks orphaned `.duet-partial-<rand>` staging files. Original source files remain 100% intact. User is prompted to resume or cancel.
- **Interrupted Cross-Device `MoveFile`:** Source file is **never unlinked** until the destination staging file is fully `fdatasync()`ed and renamed. On crash, source remains intact at original path; partial destination is cleaned.
- **Interrupted `RemoveFile` (Trash):** Atomic rename into `~/.local/share/Trash`. File is either completely at source or completely in trash.

---

## Pros and Cons of the Options

### Write-Ahead Journaling + Partial Staging (Chosen)

- **Good:** 100% crash safety; zero partially overwritten destination files; deterministic startup recovery; complete source file preservation on cross-device moves or `ENOSPC`.
- **Bad:** Slight disk I/O overhead from `fsync()` journal flushes; target directories temporarily host hidden `.duet-partial-*` files during transfer.

### Direct In-Place Writes (Rejected)

- **Good:** Slightly simpler code path; no staging file creation.
- **Bad:** Mid-transfer crashes corrupt target files; cross-device moves risk unlinking source files before destination is durable; violates `NFR-08`.

---

## Consequences

### Positive

- Satisfies `NFR-08` and passes the full automated crash injection test suite.
- Users never suffer silent data truncation or corrupted target files on unexpected power loss or `SIGKILL`.
- Provides undo history and job resume capability across application restarts.

### Negative / Risks

- Requires temporary free disk space for staging files during copy operations on full volumes.

---

## Implementation & Architecture Details

- **Journal Storage Directory:** `~/.local/state/duet/jobs/<job_id>.journal`
- **Staging Naming Format:** `.duet-partial-<32bit_rand_hex>`
- **Recovery Engine:** `duet_ops::journal::RecoveryScanner` in [`crates/duet-ops`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ops).
- **Proof Sketch:** See full invariant matrix in [`docs/crash-safety.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/crash-safety.md).

---

## Validation Strategy

- **Phase 10 Fault Injection Suite ([`docs/crash-safety.md` §5](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/crash-safety.md#L79-L86)):**
  1. `SIGKILL` Injection Loop: Process killed at randomized microsecond offsets during transfer operations, asserting zero source loss and clean staging file removal.
  2. `ENOSPC` Loop Device Harness: Small loopback volume filling mid-copy, verifying queue pause and zero target corruption.
  3. `dm-flakey` Device Mapper: Simulated I/O block drops during writes.
