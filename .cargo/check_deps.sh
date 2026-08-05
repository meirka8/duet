#!/usr/bin/env bash
set -euo pipefail

# GPUI Dependency Isolation CI Check Script (ADR-002 / T-2.1.2)
# Asserts that ONLY allowed UI crates (duet-ui, duet-widgets, duet) depend on GPUI ecosystem libraries.

ALLOWED_UI_CRATES="duet-ui duet-widgets duet"
FORBIDDEN_DEPS="gpui gpui-component gpui-macros gpui_util gpui_sum_tree gpui_refineable"

VIOLATIONS=0
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Running GPUI Dependency Isolation Check across workspace..."

for cargo_toml in "$ROOT_DIR"/crates/*/Cargo.toml "$ROOT_DIR"/plugins-sdk/Cargo.toml; do
    [ -f "$cargo_toml" ] || continue
    crate_dir="$(dirname "$cargo_toml")"
    crate_name="$(basename "$crate_dir")"

    # Skip allowed UI crates
    if [[ " $ALLOWED_UI_CRATES " =~ " $crate_name " ]]; then
        echo "  [OK] Allowed UI crate: $crate_name"
        continue
    fi

    echo "  [Checking] Core crate: $crate_name"
    for forbidden in $FORBIDDEN_DEPS; do
        if grep -E "^[[:space:]]*${forbidden}[[:space:]]*=" "$cargo_toml" > /dev/null 2>&1; then
            echo "  [ERROR] Core crate '$crate_name' ($cargo_toml) violates ADR-002 by depending on '$forbidden'!"
            VIOLATIONS=$((VIOLATIONS + 1))
        fi
    done
done

if [ "$VIOLATIONS" -gt 0 ]; then
    echo "❌ FAILED: $VIOLATIONS GPUI isolation violation(s) detected!"
    exit 1
fi

echo "✅ SUCCESS: GPUI dependency isolation enforced across all core crates."
exit 0
