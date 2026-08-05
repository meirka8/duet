# Duet Plugin SDK (`duet-plugin-sdk`)

`duet-plugin-sdk` provides WebAssembly (WASM) Component Model WIT interface definitions and Rust bindings for building extensions for the **Duet** file manager.

## Security Architecture: Zero Ambient Authority

Duet plugins run in a sandboxed WebAssembly component environment powered by Wasmtime with **Zero Ambient Authority**:
- Plugins do **not** have filesystem paths or ambient network access.
- Access to files is strictly mediated through host-granted handle IDs or capability checks.
- Memory is capped per plugin instance (default 64MB).
- Execution is strictly bounded by fuel/epoch interruption limits (default 2s per synchronous call).

## Plugin Interfaces (`duet:plugin@0.1.0`)

The WIT specification defined in `wit/duet.wit` exposes five core plugin classes matching Total Commander plugin semantics:

1. **Host (`host`)**: Shared interface for logging, progress reporting, secret retrieval, and reading granted handle streams.
2. **Content (`content-plugin`)** (WDX): Custom columns and metadata extraction.
3. **Packer (`packer-plugin`)** (WCX): Custom archive format reading, listing, extraction, and writing.
4. **Filesystem (`fs-plugin`)** (WFX): Virtual and remote filesystem providers (SFTP, WebDAV, custom protocols).
5. **Viewer (`viewer-plugin`)** (WLX): Quick view file preview renderers (Markdown, Plain Text, RGBA Canvas).
6. **Command (`command-plugin`)**: Action palette and custom shortcut command implementations.

## Getting Started

To write a plugin for Duet:

1. Target `wasm32-wasip1` or `wasm32-unknown-unknown`.
2. Add `duet-plugin-sdk` and `wit-bindgen` to your `Cargo.toml`.
3. Implement the required `Guest` traits and export your plugin using `wit_bindgen::export!`.

Refer to [`examples/stub-plugin`](examples/stub-plugin) for a complete example implementing all five plugin interfaces.
