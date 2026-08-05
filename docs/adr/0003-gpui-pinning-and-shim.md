# ADR-0003: Strict GPUI Version Pinning and `gpui-compat` Shim Strategy

- **Status:** Accepted
- **Deciders:** Lead Architect, Frontend Team
- **Date:** 2026-08-05
- **Technical Story:** Task `T-2.7.1` / [`design.md` §3.1](file:///run/media/meirk/storage_2/projects/double_manager/duet/documentation/design.md#L260) / Risk `R-G1`

---

## Context and Problem Statement

GPUI is actively developed by Zed Industries alongside the Zed editor. Because GPUI has not reached a stable 1.0 semver release, upstream updates frequently introduce breaking API changes, modified element tree builder signatures, revised context (`cx`) event dispatch methods, and altered keybinding subscription models.

If Duet relies on loose version specifications (such as `^0.2` or `latest`), routine `cargo update` runs will break binary compilation or introduce subtle runtime visual regressions. Furthermore, scattering raw GPUI calls throughout the UI layer increases maintenance debt whenever upstream APIs evolve.

---

## Decision Drivers

- **Build Reproducibility:** Ensure workspace compilation is $100\%$ deterministic across developer workstations and CI/CD pipelines.
- **Controlled Dependency Upgrade Cadence:** Prevent unintentional breaking API upgrades during feature development.
- **Minimized Refactoring Surface:** Isolate framework API changes behind a centralized compatibility layer to simplify future GPUI upgrades.

---

## Decision Outcome

**Chosen Policy:** Strict Exact Version Pinning paired with an internal `gpui-compat` Shim Module.

### 1. Exact Version Pinning in Workspace `Cargo.toml`

Dependencies on `gpui` and `gpui-component` in [`Cargo.toml`](file:///run/media/meirk/storage_2/projects/double_manager/duet/Cargo.toml) must use exact version specifiers with leading `=` signs:

```toml
[workspace.dependencies]
gpui = "=0.2.2"
gpui-component = "=0.5.1"
```

Automatic minor or patch version floating is strictly prohibited in `Cargo.toml` and `Cargo.lock`.

### 2. Internal `gpui-compat` Shim Module

All churn-prone GPUI interactions (view contexts, table delegates, keyboard input wrappers, and element tree adapters) are wrapped within an internal compatibility shim module (`duet_ui::compat` or `gpui-compat`).

```
duet-ui presentation widgets
       │
       ▼
 ┌───────────────┐
 │  gpui-compat  │  <-- Encapsulates upstream GPUI API breaking changes
 └───────┬───────┘
         ▼
    gpui crate
```

### 3. Deliberate Version Upgrade Gate Protocol

GPUI version upgrades are executed as dedicated, scheduled maintenance tasks rather than passive updates:
1. Open a dedicated upgrade branch (`feature/deps-gpui-bump-x.y.z`).
2. Update the exact version pin in `Cargo.toml`.
3. Adjust `gpui-compat` shim implementations to resolve upstream API changes.
4. Pass full compile, unit test, and visual smoke test gates before merging into main branches.

---

## Pros and Cons of the Options

### Strict Version Pinning + `gpui-compat` Shim (Chosen)

- **Good:** Completely eliminates surprise build breakages from upstream crates; centralizes upstream API diffs to `gpui-compat`; provides predictable upgrade cycles; shields UI widgets from direct API churn.
- **Bad:** Delay in receiving upstream performance improvements or bug fixes until explicit upgrade tasks are scheduled; requires maintaining internal shim abstractions.

### Floating SemVer Dependencies (`gpui = "0.2"`)

- **Good:** Automatically picks up upstream bug fixes and performance enhancements on `cargo update`.
- **Bad:** Frequent CI build failures due to unannounced upstream breaking changes; spreads framework churn throughout all widget code.

---

## Consequences

### Positive

- Zero unannounced compilation failures caused by upstream GPUI releases.
- Upgrading GPUI versions requires modifying only `gpui-compat` and validating UI delegates.
- Clean separation of concern within [`duet-ui`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui).

### Negative / Risks

- Requires developer effort to periodically triage upstream Zed GPUI releases and plan version bump tasks.

---

## Implementation & Architecture Details

- **Location:** [`crates/duet-ui/src/compat/`](file:///run/media/meirk/storage_2/projects/double_manager/duet/crates/duet-ui)
- **Shim Responsibilities:**
  - Standardized `RenderContext` and `WindowContext` wrapper helpers.
  - Uniform event subscriber registration for TC keybindings (`FR-CFG-02`).
  - Virtualized table delegate adapters bridging Struct-of-Arrays data pools ([ADR-0005](file:///run/media/meirk/storage_2/projects/double_manager/duet/docs/adr/0005-soa-directory-memory-layout.md)) to `gpui-component::table`.

---

## Validation Strategy

- **`Cargo.lock` Verification:** CI check asserting exact equality of `gpui` crate version against pinned manifest version.
- **Upgrade PR Gate:** Automated CI test matrix executing visual smoke tests and keybinding validation on upgrade branches.
