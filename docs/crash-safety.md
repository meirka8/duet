# Duet Crash-Safety Proof Sketch & Invariant Matrix (T-2.3.2)

## 1. Executive Summary & Core Safety Invariant

Duet is designed as a crash-safe orthodox file manager. Data modification commands in `duet-ops` execute under an append-only, fsync'd write-ahead journal (`~/.local/state/duet/jobs/<job_id>.journal`) and double-buffered partial staging (`.duet-partial-*`).

### Core Invariant (NFR-08 / §9.3)
> **For every fault point during any file operation step, either the source is completely intact and the destination is absent or clearly marked partial (`.duet-partial-<rand>`), or the destination is complete, verified, and durable (`fsync`'d). Never a silently truncated or partially overwritten destination file.**

---

## 2. Operation Step & Staged Execution Lifecycle

Every execution job is decomposed into concrete, atomic `Step` variants by the operation planner (`Plan`).

| Step Kind | Description | Staging Mechanism | Atomic Commit |
|---|---|---|---|
| `CreateDir` | Create target directory structure with initial permissions. | Direct directory creation (`mkdirat`) | `fstat` check |
| `CopyFile` | Read content from source, write to destination. Uses copy ladder (reflink $\to$ `copy_file_range` $\to$ sparse-buffered). | Staged target: `.duet-partial-<rand>` | `fdatasync` + `renameat2` |
| `MoveFile` | Move source file/directory to destination. | Same-dev: `renameat2`. Cross-dev: staged copy + verify + unlink source. | Same-dev: single syscall. Cross-dev: `fsync` dest before source `unlink`. |
| `RemoveFile` | Remove file or directory (trash staging or permanent delete). | Trash: `renameat2` into `~/.local/share/Trash`. Permanent: journal pre-record then `unlinkat`. | Journal entry `StepStarted` fsync'd before call. |
| `SetMetadata` | Apply file permissions (`chmod`), ownership (`chown`), xattrs, POSIX ACLs, SELinux labels, and timestamps (`utimensat`). | Metadata updated in sequence: mode $\to$ xattr $\to$ ACL $\to$ mtime/atime last. | Individual syscalls. Timestamps last prevent ctime distortion. |
| `ApplyPatch` | Apply diff/patch stream to a file. | Staged file `.duet-partial-<rand>` written with patched content. | `fdatasync` + `renameat2` |
| `Truncate` | Sizing / truncating target file. | Staged copy `.duet-partial-<rand>` truncated to specified length. | `fdatasync` + `renameat2` |
| `AtomicRename` | Rename source to target path atomically. | Kernel `renameat2(RENAME_NOREPLACE)` or `renameat2(0)` | Single kernel syscall |
| `Reflink` | Zero-copy reflink (`FICLONE` / `FICLONERANGE`) copy. | Staged target `.duet-partial-<rand>` reflinked. | `renameat2` |
| `CreateSymlink` | Create symbolic link. | Temporary symlink `.duet-partial-<rand>` | `renameat2` |
| `CreateHardlink` | Preserves hardlink inode graph across operation set. | Linked to target path or staging path. | `linkat` |
| `VerifyChecksum` | Post-copy CRC32 / BLAKE3 verification. | Read source and staging target, compare digests. | Step fails before final `renameat2` if mismatch. |

---

## 3. Write-Ahead Journal Protocol (`~/.local/state/duet/jobs/*.journal`)

Each job maintains an append-only journal file with the following state machine:

```
[JobStarted] ──> [StepStarted(step_id)] ──> [StepProgress(offset)] ──> [StepCompleted(step_id)] ──> [JobCompleted]
                       │                                                     ▲
                       └──> (Interruption Point / SIGKILL / ENOSPC) ─────────┘ (Recovery Scanner)
```

1. **Write-Ahead Intent**: Before executing any step that modifies state, `StepStarted(step_id, src, dst, staged_dst)` is written to the job journal and **`fsync()`ed to disk**.
2. **Partial Staging**: Payload data is written exclusively to `staged_dst` (`<dst_parent>/.duet-partial-<rand>`).
3. **Data Durability**: When payload transfer completes, `fdatasync()` is called on `staged_dst`.
4. **Atomic Swap**: `renameat2(staged_dst, dst)` atomically replaces the destination.
5. **Step Completion**: `StepCompleted(step_id)` is appended to the journal and flushed.

---

## 4. Interruption Point Invariant Matrix

The table below enumerates every `Step` kind against all potential fault points, defining the invariant state and mapping to Phase 3 / Phase 10 validation test cases.

| Step Kind | Interruption Point | Pre-Fault State | Post-Fault State (Invariant) | Journal State | Recovery Action | Test Case Path |
|---|---|---|---|---|---|---|
| `CopyFile` | `SIGKILL` mid-transfer | Source intact, target absent/old | Source intact, target `.duet-partial-*` exists. Target intact if already renamed. | `StepStarted` or `StepProgress` | Scanner prompts user; cleans partial file or resumes from byte offset (`APPEND_RESUME`). | `tests/crash_safety/test_copy_file_sigkill.rs` |
| `CopyFile` | `ENOSPC` mid-transfer | Source intact, partial target written | Source intact, partial file `.duet-partial-*` remains on target volume. Partial space freed on abort. | `StepProgress` logged | Queue pauses globally; user frees space or cancels; partial cleaned up. | `tests/crash_safety/test_copy_file_enospc.rs` |
| `CopyFile` | Power Loss / System Crash | Source intact, dirty pages in page cache | Host reboot: source intact. Partial file `.duet-partial-*` truncated/zeroed or incomplete. Destination untouched. | `StepStarted` flushed | Startup scanner detects unclosed journal, unlinks orphaned `.duet-partial-*`. | `tests/crash_safety/test_copy_file_power_loss.rs` |
| `CopyFile` | `EACCES` mid-transfer | Source intact, target partial created | Source intact, partial target exists. Permission error caught. | `StepProgress` logged | Prompt user for elevated Polkit retry (`duet-privileged`) or skip. | `tests/crash_safety/test_copy_file_eacces.rs` |
| `CopyFile` | Network Disconnect | Source intact (remote), local partial written | Source intact on remote, local `.duet-partial-*` preserved. | `StepProgress` logged | Retry with exponential backoff; resume transfer from last verified offset. | `tests/crash_safety/test_copy_file_net_disconnect.rs` |
| `CopyFile` | `dm-flakey` I/O Error | Source intact, block write fails | Source intact, target `.duet-partial-*` incomplete. Original target intact. | `StepProgress` logged | Mark step failed; preserve original file; prompt user. | `tests/crash_safety/test_copy_file_io_error.rs` |
| `CreateDir` | `SIGKILL` / Crash | Path absent | Either directory created with target mode or absent. Operation is idempotent (`mkdir -p`). | `StepStarted` logged | Resume scanner re-checks directory existence; continues. | `tests/crash_safety/test_create_dir_crash.rs` |
| `MoveFile` (Same-Dev) | `SIGKILL` mid-rename | Source at `src`, target at `dst` | Atomic `renameat2`: source is at `src` OR source is at `dst`. Never both, never half-moved. | `StepStarted` logged | Scanner verifies location of file; updates job record. | `tests/crash_safety/test_move_samedev_sigkill.rs` |
| `MoveFile` (Cross-Dev) | `SIGKILL` mid-copy | Source at `src`, target absent | Source at `src` intact. Partial target `.duet-partial-*` on dest device. `src` is NEVER unlinked until `dst` is `fsync`'d. | `StepStarted` logged | `src` preserved. Delete partial target on destination; retry or cancel. | `tests/crash_safety/test_move_crossdev_sigkill.rs` |
| `RemoveFile` (Trash) | `SIGKILL` mid-trash | File at `src` | Atomic rename to trash dir. File is at `src` OR in trash. Never deleted. | `StepStarted` logged | Resume checks file location; updates undo stack. | `tests/crash_safety/test_remove_trash_sigkill.rs` |
| `RemoveFile` (Permanent) | `SIGKILL` mid-unlink | File at `src` | Journal entry `StepStarted` logged before `unlink`. Either file unlinked or intact at `src`. | `StepStarted` flushed | Scanner checks if file exists; logs completion or re-executes. | `tests/crash_safety/test_remove_perm_sigkill.rs` |
| `SetMetadata` | `SIGKILL` mid-metadata | File content intact, metadata default/old | Metadata updated in atomic syscall steps (mode $\to$ xattr $\to$ ACL $\to$ timestamps last). File content unaffected. | `StepStarted` logged | Re-apply metadata sequence to target. | `tests/crash_safety/test_set_metadata_sigkill.rs` |
| `ApplyPatch` | `SIGKILL` / ENOSPC | Original file intact | Original file untouched. Patched content in `.duet-partial-*`. | `StepStarted` logged | Delete partial patch file; original intact. | `tests/crash_safety/test_apply_patch_crash.rs` |
| `Truncate` | `SIGKILL` mid-truncate | Original file intact | Original file untouched. Truncated staging copy `.duet-partial-*`. | `StepStarted` logged | Delete partial staged file. | `tests/crash_safety/test_truncate_crash.rs` |
| `AtomicRename` | Power Loss / Crash | Source at `src`, target at `dst` | `renameat2` is atomic in POSIX filesystem. Path is at `src` OR `dst`. | `StepStarted` logged | Scanner inspects paths, reconciles journal state. | `tests/crash_safety/test_atomic_rename_crash.rs` |
| `Reflink` | `SIGKILL` / Fail | Source intact | Reflink target `.duet-partial-*` created or absent. On failure, fall back to copy ladder. | `StepStarted` logged | Cleanup partial target; retry via `copy_file_range` or sparse copy. | `tests/crash_safety/test_reflink_crash.rs` |
| `CreateSymlink` | `SIGKILL` | Symlink absent | Symlink created or absent. Idempotent replace via staging. | `StepStarted` logged | Cleanup partial symlink if present; retry. | `tests/crash_safety/test_create_symlink_crash.rs` |
| `CreateHardlink` | `SIGKILL` | Hardlink absent | Target linked to inode or absent. Inode link count $\ge 1$. | `StepStarted` logged | Check link count and inode table; resume. | `tests/crash_safety/test_create_hardlink_crash.rs` |
| `VerifyChecksum` | Mismatch / Fault | Staged target complete | Mismatch detected $\to$ staging target `.duet-partial-*` unlinked. Final destination NOT replaced. | `StepStarted` logged | Journal logs checksum failure (`FailedChecksum`); user notified. | `tests/crash_safety/test_verify_checksum_fault.rs` |

---

## 5. Phase 10 Data-Safety Verification Suite (`T-10.2.1`)

All scenarios in Section 4 will be continuously verified by the automated fault-injection suite in `tests/crash_safety/`:

1. **Loop Device `ENOSPC` Harness**: Mounts a small (10 MiB) tmpfs/loopback ext4 volume. Executes multi-file copy/move jobs to trigger `ENOSPC` mid-operation. Asserts queue pause, zero corruption, and partial file cleanup on cancel.
2. **`dm-flakey` Device Mapper Harness**: Simulates random block-level read/write failures and mid-transfer disconnects on FUSE/remote mounts.
3. **`SIGKILL` Injection Loop**: Runs jobs in a child process, issuing `SIGKILL` at randomized microsecond offsets during file operations. Asserts startup recovery scanner cleans all partials and restores clean state with zero lost source files.
