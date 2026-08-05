# ADR-0002: Architectural UI Isolation Boundary

- **Status:** Accepted
- **Deciders:** Lead Architect, Core Infrastructure Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §7.5](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L232) / [`AGENTS.md` §4.1](file:///run/media/meirk/storage_2/projects/double_manager/duet/AGENTS.md#L49-L52)

---

## Context and Problem Statement

Adopting a pre-1.0 rendering framework such as GPUI ([ADR-0001](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0001-gpui-ui-framework.md)) exposes the project to breaking API changes, upstream refactoring churn, and potential framework replacement if desktop integration hurdles arise. Furthermore, coupling filesystem operations, indexing, and VFS traits directly to UI types makes headless testing impossible, degrades UI thread responsiveness, and prevents building alternative frontends (e.g., TUI or headless CLI automation).

The architecture requires a strict isolation boundary between the visual shell and the core file management engine.

---

## Decision Drivers

- **Framework Agnosticism:** Core file management engine must have zero dependency on GPUI, Wayland, X11, or visual rendering toolkits.
- **Headless Testability:** 100% of VFS backends, job journaling, sorting algorithms, and search engines must be testable in headless CI environments without a display server or GPU renderer.
- **UI Thread Safety:** Prevent blocking filesystem I/O or long-running syscalls from ever running on the main UI loop (`NFR-02`).
- **Bounded Refactoring Cost:** If GPUI must be replaced (e.g., fallback to Iced), the refactoring scope must be limited strictly to the visual presentation layer (~25% of codebase).

---

## Decision Outcome

**Chosen Policy:** Strict Layered Architecture with Hard Dependency Barriers.

1. **Only `duet-ui` and `duet-widgets` may import `gpui` or `gpui-component`.**
2. **All core engine crates (`duet-types`, `duet-vfs`, `duet-ops`, `duet-index`, `duet-search`, `duet-meta`, `duet-config`, `duet-plugin`, `duet-platform`, `duet-commands`) must remain completely UI-agnostic.**

```
┌────────────────────────────────────────────────────────┐
│               duet-ui / duet-widgets                   │
│          (GPUI rendering, Views, Delegates)            │
└──────────────────────────┬─────────────────────────────┘
                           │ Imports / Calls
                           ▼
┌────────────────────────────────────────────────────────┐
│             duet-types / Core Engine                   │
│   (Plain Data Structures, VFS Traits, Job Engine)     │
└────────────────────────────────────────────────────────┘
  ▲ NO GPUI DEPENDENCY ALLOWED IN CORE CRATES
```

---

## Cross-Layer Communication Mechanics

Cross-layer interaction between the core engine and the GPUI presentation layer relies on three UI-agnostic primitives:

1. **Plain Data Transfer Objects (DTOs):** All directory entries, job progress updates, configuration settings, and VFS metadata are defined as standard Rust structs/enums in [`duet-types`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-types).
2. **Async Channels & Streams:** `tokio::sync::mpsc` channels and `futures::Stream` instances stream file enumeration chunks, search hits, and operation progress from background Tokiod tasks to the UI event loop.
3. **Abstract `Executor` Trait:** Core operations trigger background execution through a minimal async spawning trait implemented by the UI layer:

```rust
pub trait TaskExecutor: Send + Sync {
    fn spawn_background(&self, fut: Pin<Box<dyn Future<Output = ()> + Send>>);
}
```

---

## Pros and Cons of the Options

### Strict UI Isolation Boundary (Chosen)

- **Good:** Complete headless testability in headless CI pipelines; core engine reusable for future TUI/CLI tools; UI framework swap cost strictly bounded to presentation crates; guarantees UI thread remains non-blocking (`T-3.1.6`).
- **Bad:** Requires boilerplate mapping between core DTOs and GPUI view entities; requires explicit channel orchestration for async updates.

### Direct Coupling (Core importing GPUI types/models)

- **Good:** Less initial mapping boilerplate; direct access to GPUI state signals.
- **Bad:** Headless testing fails without mock display servers; core engine locked to GPUI API churn; risk of accidentally executing disk I/O on GPUI main thread; framework migration requires re-writing the entire codebase.

---

## Consequences

### Positive

- Core engine crates achieve $100\%$ headless test coverage in Linux containerized CI environments.
- Enforces strict UI thread discipline: no disk `stat`, network I/O, or heavy sorting on GPUI event loop thread.
- Facilitates modular replacement or parallel implementation of alternative frontends (e.g., TUI).

### Negative / Risks

- DTO translation overhead between `duet-types` and GPUI view delegates (mitigated by zero-copy Struct-of-Arrays indexing in [ADR-0005](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0005-soa-directory-memory-layout.md)).

---

## Implementation & Architecture Details

- **Workspace Dependency Rules:** Root [`Cargo.toml`](file:///run/media/meirk/storage_2/projects/double_manager/duet/Cargo.toml) declares `gpui` and `gpui-component` under `workspace.dependencies`. Only [`crates/duet-ui/Cargo.toml`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui/Cargo.toml) and [`crates/duet-widgets/Cargo.toml`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-widgets/Cargo.toml) are permitted to reference these workspace dependencies.
- **CI Dependency Guard:** Automated CI step checks `cargo tree -p duet-vfs -p duet-ops -p duet-types --invert gpui` asserting zero matching nodes.

---

## Validation Strategy

- **Static CI Linting:** Automated script running `cargo tree` to ensure `gpui` is absent from non-UI crate dependency graphs.
- **Headless Unit Tests:** All `duet-vfs`, `duet-ops`, and `duet-index` tests execute in headless environment (`DISPLAY=` and `WAYLAND_DISPLAY=` unset).
- **UI Thread Blocking Guard (`T-3.1.6`):** Instrumentation guard asserting zero synchronous file syscalls on the main event thread.
