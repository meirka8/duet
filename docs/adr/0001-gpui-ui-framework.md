# ADR-0001: Choice of GPUI as GPU-Accelerated Rendering Framework

- **Status:** Accepted (Conditional on Phase 0 Spikes S-1…S-6)
- **Deciders:** Lead Architect, UX & Core Systems Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §3.1](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L225-L270) / `NFR-01`..`NFR-06`

---

## Context and Problem Statement

Duet is designed as a high-performance, GPU-accelerated, keyboard-first orthodox file manager for modern Linux desktops ([`design.md`](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md)). To achieve parity with or surpass legacy tools (such as Total Commander and Krusader) while feeling native on modern high-refresh-rate Linux displays ($120\text{ Hz}$ to $240\text{ Hz}$), the choice of UI rendering framework must satisfy extreme performance bounds:

1. **Cold Start to Interactive (`NFR-01`):** $\le 150\text{ ms}$ on warm page cache.
2. **Keystroke-to-Pixel Latency (`NFR-02`):** $p50 \le 6\text{ ms}$, $p99 \le 12\text{ ms}$.
3. **Large Scale Listings (`NFR-04`):** Scrollable and sortable directory listings of 1,000,000 entries loaded $\le 3\text{ s}$ with zero UI stalls ($> 16\text{ ms}$).
4. **Sustained Scroll Frame Rates (`NFR-05`):** Sustained $120\text{ Hz}$ refresh rate (frame rendering budget $< 8.33\text{ ms}$) without per-frame memory allocations.
5. **Memory Footprint (`NFR-06`):** $\le 150\text{ MB}$ RSS with dual panes loaded at 100,000 entries each.
6. **No Heavy Desktop Runtime Dependencies (`NFR-10`):** Must execute on minimal Wayland or X11 environments without requiring heavy GTK or Qt runtime stacks.

Traditional GUI frameworks fail to satisfy these combined requirements under high-row-count virtualized tables or introduce heavy runtime/licensing trade-offs.

---

## Decision Drivers

- **Hardware Acceleration:** Native GPU rendering (Vulkan/Blade) with quad and text batching to maintain $120\text{ Hz}$ frame rates.
- **Native Rust Integration:** High memory safety, seamless async concurrency with Tokio/Futures, zero FFI overhead.
- **Virtualized Table Performance:** Ability to render $1,000,000$ row virtual tables with sub-millisecond layout calculations.
- **Keyboard-First Event Routing:** Precise, low-latency keybinding dispatch engine suitable for Total Commander muscle memory compatibility (`FR-CFG-02`).
- **Minimal Resource Footprint:** Low binary size ($\le 40\text{ MB}$) and tight memory overhead ($\le 150\text{ MB}$ RSS).

---

## Considered Options

1. **GPUI + `gpui-component` (Chosen):** The GPU-accelerated retained-entity view framework built by Zed Industries in Rust. Uses Vulkan/Blade for hardware rendering with immediate view execution and quad/text batching.
2. **GTK4 / Libadwaita (`gtk4-rs`):** Standard GNOME UI toolkit. Retained scene graph with CPU/Cairo/GSK rendering.
3. **Qt 6 / QML (`CXX` or `qmeta-async`):** Mature cross-platform C++ GUI framework with custom QML/OpenGL rendering pipeline.
4. **Iced:** Native Rust cross-platform GUI framework based on the Elm architecture with WebRender/wgpu backends.
5. **Tauri / Electron:** Web-technology based shell using WebAssembly/HTML5 Canvas/DOM rendered in WebKitGTK / Chromium.

---

## Decision Outcome

**Chosen Option:** **GPUI + `gpui-component`** for the `duet-ui` and `duet-widgets` shell.

### Justification

GPUI is specifically architected for code editors and project trees handling millions of lines and elements. Its GPU batching pipeline (quad/text primitives pushed directly to Vulkan command buffers) enables ultra-low input latency ($p50 \le 6\text{ ms}$) and sustained $120\text{ Hz}$ rendering during rapid scrolling over $1,000,000$-entry datasets. Combining GPUI with `gpui-component` provides standard table delegates and component primitives tailored for pure Rust async execution.

This decision is conditional on passing Phase 0 feasibility spikes (`S-1` table scale, `S-2` clipboard `text/uri-list`, `S-3` Wayland DnD). If critical Wayland desktop integrations fail without viable workarounds, **Iced** serves as the documented fall-back substitute, bounded by the architectural isolation mandated in [ADR-0002](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0002-ui-isolation-boundary.md).

---

## Pros and Cons of the Options

### GPUI + `gpui-component`

- **Good:** Hardware-accelerated text and quad batching via Vulkan/Blade; sub-frame keystroke-to-pixel latency; pure Rust entity model; excellent memory efficiency fitting well within $\le 150\text{ MB}$ RSS (`NFR-06`).
- **Bad:** Pre-1.0 crate under active development by Zed with frequent API breaks; lack of native AT-SPI Linux accessibility tree (AccessKit integration is a tracked gap for `NFR-11` / `OQ-4`).

### GTK4 / Libadwaita

- **Good:** Native Linux HIG appearance; mature accessibility (AT-SPI2) and IME support; established Wayland clipboard/DnD integration.
- **Bad:** High RSS memory usage ($> 250\text{ MB}$); complex custom cell virtualization for $1\text{M}$ rows; GObject C-FFI wrapper overhead (`gtk4-rs`); strict main-thread affinity causing UI freezes if event dispatch delays occur.

### Qt 6 / QML

- **Good:** Exceptional table view performance (`QTableView` / `QAbstractItemModel`); robust Wayland and X11 support.
- **Bad:** Heavy C++ FFI interop complexity (`cxx` / `qmeta-async`); LGPLv3 / Commercial licensing constraints; large runtime binary footprint violating `NFR-09` ($\le 40\text{ MB}$).

### Iced

- **Good:** Pure Rust Elm architecture; active community; modular `wgpu` rendering backend; clean state model.
- **Bad:** Lacks native hardware-accelerated text batching at Zed/GPUI scale; layout calculations for $1\text{M}$ virtual rows introduce measurable CPU overhead; custom delegate styling requires extensive framework extensions.

### Tauri / Electron

- **Good:** Rapid layout prototyping with standard CSS/HTML.
- **Bad:** Fails `NFR-01` (cold start $> 400\text{ ms}$), `NFR-02` (latency $> 30\text{ ms}$), and `NFR-06` (memory $> 350\text{ MB}$ RSS); IPC serialization bottleneck when streaming $1,000,000$ directory records.

---

## Consequences

### Positive

- Enables $120\text{ Hz}$ ultra-smooth scrolling and sub-$6\text{ ms}$ keystroke response across dual file panes.
- Zero dependency on GTK/Qt runtime libraries (`NFR-10`).
- Binary size remains compact ($\le 40\text{ MB}$).

### Negative / Risks

- Rapid upstream GPUI API evolution requires strict version pinning ([ADR-0003](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0003-gpui-pinning-and-shim.md)).
- Linux AT-SPI accessibility tree must be addressed post-1.0 via AccessKit integration ([`design.md` §17 OQ-4](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L729)).

---

## Implementation & Architecture Details

- **UI Shell Crate:** Restricted strictly to [`duet-ui`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui) and [`duet-widgets`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-widgets).
- **Core Engine Isolation:** Core engine crates (`duet-vfs`, `duet-ops`, `duet-types`) do **not** import `gpui` or `gpui-component`.

---

## Validation Strategy

- **`NFR-01` Benchmark:** `hyperfine` measurements of binary startup to first frame paint on warm page cache.
- **`NFR-02` Benchmark:** In-process event-to-frame timestamp recording asserting $p50 \le 6\text{ ms}$.
- **`NFR-05` Frame Histogram:** Continuous CI scroll benchmark on $1,000,000$-entry generated datasets asserting zero frame times $> 8.33\text{ ms}$.
