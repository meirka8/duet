# ADR-0004: WASM Component Model Plugins Over Native Shared Libraries

- **Status:** Accepted
- **Deciders:** Lead Architect, Security & Extension Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §3.1](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L262) / [`AGENTS.md` §1](file:///run/media/meirk/storage_2/projects/double_manager/duet/AGENTS.md#L18) / [`plugins-sdk`](file:///run/media/meirk/storage_2/projects/double_manager/duet/plugins-sdk)

---

## Context and Problem Statement

An orthodox file manager requires an extensible plugin ecosystem to support custom archive formats, remote VFS protocols, custom metadata columns, and previewers (analogous to Total Commander's `.wcx`, `.wfx`, `.wdx`, and `.wlx` plugins).

Historically, file managers load native C/C++ dynamic libraries (`.so` on Linux, `.dll` on Windows) directly into the host process memory space via `dlopen()`. However, native shared libraries present severe security, stability, and compatibility hazards:

1. **Zero-Day Security Vulnerabilities:** Arbitrary native code execution with ambient host credentials (access to user SSH keys, GPG tokens, personal files).
2. **Host Instability:** Memory corruption (use-after-free, double-free, null pointer dereference) in a plugin crashes the entire file manager process.
3. **ABI Instability:** Rust lacks a stable C++ ABI; raw C-FFI interfaces require manual memory management and introduce cross-compiler version incompatibilities.
4. **Cross-Platform Portability:** Native plugins must be compiled separately for x86_64, aarch64, and RISC-V architectures.

---

## Decision Drivers

- **Security & Sandboxing:** Enforce strict Zero Ambient Authority — plugins must have zero implicit access to the host filesystem, network, or environment.
- **Process Isolation & Crash Resilience:** A panicking or buggy plugin must never crash or corrupt the Duet host manager process.
- **Resource Control:** Host must cap plugin CPU consumption (fuel metering) and memory allocation limits.
- **Cross-Platform Portability:** Plugin binaries should execute identically across CPU architectures without re-compilation.
- **Language Independence:** Support plugin development in Rust, C/C++, Go, Python, and AssemblyScript.

---

## Considered Options

1. **WASM Component Model via Wasmtime (Chosen):** Execute plugins as WebAssembly components conforming to WebAssembly Interface Type (WIT) specifications using the `wasmtime` runtime.
2. **Native Shared Libraries (`.so` / `dlopen`):** Load native C-ABI dynamic libraries directly into host memory space.
3. **Out-of-Process Native RPC Plugins:** Run third-party binaries as separate subprocesses communicating over IPC pipes via Protocol Buffers / gRPC.

---

## Decision Outcome

**Chosen Option:** **WASM Component Model via Wasmtime** implemented in [`crates/duet-plugin`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-plugin) and [`plugins-sdk`](file:///run/media/meirk/storage_2/projects/double_manager/duet/plugins-sdk).

### Security Architecture

- **Zero Ambient Authority:** WASM binaries run in a strict sandbox. Filesystem paths, network sockets, and environment variables are accessible only when explicitly granted by host capabilities defined in WIT files.
- **Fuel Metering & Epoch Interrupts:** Wasmtime fuel consumption counters prevent infinite loops; epoch interrupts bound long-running computation.
- **Memory Isolation:** Each plugin executes within its own isolated linear memory space. Out-of-bounds access triggers a sandbox trap without affecting the host process.

### Legacy Native Plugin Bridge (Deferred to Post-1.0, `OQ-7`)

Native dynamic libraries (such as legacy Total Commander `.wcx`/`.wdx` DLLs under Wine or native Linux `.so` plugins) are explicitly prohibited from running inside the main host process. Support for native binaries is deferred to a post-1.0 out-of-process isolation daemon (`duet-native-plugin-host`) communicating via IPC pipes ([`design.md` §15.2 / OQ-7](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L586)).

---

## Pros and Cons of the Options

### WASM Component Model (Chosen)

- **Good:** Strong security isolation with capability-based WASI 0.2 permissions; plugin panics trap safely without crashing Duet; cross-platform architecture independence; strict CPU fuel and memory limits; clean multi-language interface via WIT definitions.
- **Bad:** Near-native JIT overhead (~10–15% CPU execution penalty compared to raw native code); requires plugin authors to target WebAssembly.

### Native Shared Libraries (`.so` / `dlopen`)

- **Good:** Maximum execution speed; direct C-ABI interface; access to native host libraries.
- **Bad:** High security risk (full access to host user environment); plugin memory corruption crashes host process; zero resource metering; fragile ABI compatibility across compiler versions.

### Out-of-Process Native RPC

- **Good:** Process-level crash isolation; permits native binary execution.
- **Bad:** High IPC serialization overhead when exchanging dense directory listings or thumbnail bitmaps; complex process lifecycle management.

---

## Consequences

### Positive

- Duet marketplace plugins can be installed safely without root privileges or code-signing fear.
- Stable, language-agnostic plugin API defined via WIT interfaces in [`plugins-sdk`](file:///run/media/meirk/storage_2/projects/double_manager/duet/plugins-sdk).
- Host process protected against memory corruption, segfaults, and unauthorized network/disk access.

### Negative / Risks

- High-throughput VFS plugins (e.g. custom filesystem drivers) incur WASM memory boundary copy overhead (mitigated by WIT canonical ABI stream buffers).

---

## Implementation & Architecture Details

- **Host Runtime Crate:** [`crates/duet-plugin`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-plugin) incorporating `wasmtime` and `wasmtime-wasi`.
- **SDK & Specs:** [`plugins-sdk`](file:///run/media/meirk/storage_2/projects/double_manager/duet/plugins-sdk) containing WIT interface contracts for:
  - `duet:plugin/vfs`: Custom virtual filesystem drivers.
  - `duet:plugin/meta`: Custom file column and metadata extraction.
  - `duet:plugin/preview`: Text and binary preview renderers.

---

## Validation Strategy

- **Sandbox Security Conformance:** CI integration tests loading malicious WASM modules attempting unauthorized socket/path access, verifying sandboxed trapping.
- **Fuel Metering Test:** Test cases injecting infinite loops (`loop {}`) into WASM plugins, asserting host interrupt within specified fuel quotas.
