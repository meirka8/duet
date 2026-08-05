# Duet Architecture Decision Records (ADRs) Index

This directory contains the formal Architecture Decision Records (ADRs) for the **Duet** GPU-accelerated, keyboard-first orthodox file manager. All records follow standard MADR (Markdown Architecture Decision Records) / Nygard format per [`design.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md) and [`AGENTS.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/AGENTS.md).

---

## Architecture Decision Records

| ADR | Title | Status | Date | Primary Drivers & Scope | Target Crates / Specs |
|---|---|---|---|---|---|
| [**ADR-0001**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0001-gpui-ui-framework.md) | Choice of GPUI as GPU-Accelerated Rendering Framework | Accepted (Conditional on G0) | 2026-08-05 | `NFR-01`..`NFR-06`, $120\text{ Hz}$ GPU quad/text batching, sub-$6\text{ ms}$ input latency | [`duet-ui`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui), [`duet-widgets`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-widgets) |
| [**ADR-0002**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0002-ui-isolation-boundary.md) | Architectural UI Isolation Boundary | Accepted | 2026-08-05 | UI-framework agnosticism, headless CI testing, UI thread blocking guard (`T-3.1.6`) | Core engine crates (`duet-vfs`, `duet-ops`, `duet-types`) |
| [**ADR-0003**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0003-gpui-pinning-and-shim.md) | Strict GPUI Version Pinning and `gpui-compat` Shim Strategy | Accepted | 2026-08-05 | Upstream GPUI API churn mitigation, build reproducibility, Risk `R-G1` | `Cargo.toml`, [`duet-ui::compat`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui) |
| [**ADR-0004**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0004-wasm-component-plugins.md) | WASM Component Model Plugins Over Native Shared Libraries | Accepted | 2026-08-05 | Zero ambient authority security, WASI 0.2 WIT specifications, crash resilience | [`duet-plugin`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-plugin), [`plugins-sdk`](file:///run/media/meirk/storage_2/projects/double_manager/duet/plugins-sdk) |
| [**ADR-0005**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0005-soa-directory-memory-layout.md) | Struct-of-Arrays (SoA) Directory Model with String Interning | Accepted | 2026-08-05 | $1,000,000$ directory entries scale, $\le 96\text{ B}$ memory budget, $\le 150\text{ MB}$ RSS (`NFR-06`) | [`duet-types`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types), [`duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs) |
| [**ADR-0006**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0006-crash-safe-journaling.md) | Append-Only Write-Ahead Job Journaling and Partial Staging | Accepted | 2026-08-05 | Zero data loss/corruption (`NFR-08`), `SIGKILL` / `ENOSPC` recovery | [`duet-ops`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ops), [`docs/crash-safety.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/crash-safety.md) |
| [**ADR-0007**](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0007-oq5-vfs-backend-strategy.md) | VFS Backend Strategy — Hand-Rolled POSIX `LocalFs` Paired with OpenDAL Bridge | Accepted (OQ-5 Decided) | 2026-08-05 | High-performance POSIX `getdents64` streaming + `statx` (`NFR-03`, `NFR-07`) paired with OpenDAL for S3/SFTP | [`duet-vfs`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-vfs) |

---

## Architectural Guidelines Summary

1. **Isolation Rule ([ADR-0002](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0002-ui-isolation-boundary.md)):** Only `duet-ui` and `duet-widgets` may import `gpui` or `gpui-component`. Core engine crates must remain completely UI-agnostic.
2. **UI Thread Discipline:** Long-running syscalls, file stat calls, or network I/O must never block the main GPUI event loop thread.
3. **Data Safety First ([ADR-0006](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0006-crash-safe-journaling.md)):** All file operations write to job journals (`~/.local/state/duet/jobs/*.journal`) and staging files (`.duet-partial-*`) before atomic commit.
4. **Zero Ambient Authority Extensions ([ADR-0004](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0004-wasm-component-plugins.md)):** Plugins run in WASM Component sandboxes with fuel limits and capability-based WIT permissions.
5. **High-Scale Performance ([ADR-0005](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0005-soa-directory-memory-layout.md), [ADR-0007](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0007-oq5-vfs-backend-strategy.md)):** Directory tables use Struct-of-Arrays memory with string interning and direct Linux `getdents64` streaming.
