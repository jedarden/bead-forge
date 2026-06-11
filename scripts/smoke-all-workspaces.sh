#!/usr/bin/env bash
# Smoke test script for bf (bead-forge) binary
# Runs bf list and bf sync --flush-only on all active workspaces
#
# Usage: ./smoke-all-workspaces.sh
#
# This script should be run:
# 1. After installing a new bf binary (manually or via auto-deploy)
# 2. As a post-build verification step in bead-forge-build-workflowtemplate.yml
#
# Workspaces to test: bead-forge, SIGIL, HOOP, FABRIC, spaxel, miroir, pdftract,
#                    mobile-gaming, drawrace, ai-code-battle

set -euo pipefail

# List of active workspaces to test (relative to $HOME)
WORKSPACES=(
    "bead-forge"
    "SIGIL"
    "HOOP"
    "FABRIC"
    "spaxel"
    "miroir"
    "pdftract"
    "mobile-gaming"
    "drawrace"
    "ai-code-battle"
)

BF_BIN="${HOME}/.local/bin/bf"
FAILED=()
PASSED=()

echo "=== bf workspace smoke test ==="
echo "Testing $(basename "$BF_BIN") on ${#WORKSPACES[@]} workspaces"
echo ""

for workspace in "${WORKSPACES[@]}"; do
    workspace_path="${HOME}/${workspace}"

    # Skip if workspace doesn't exist or doesn't have .beads
    if [[ ! -d "$workspace_path" ]] || [[ ! -d "$workspace_path/.beads" ]]; then
        echo "⚠️  SKIP: $workspace (not found or no .beads/)"
        continue
    fi

    echo -n "Testing $workspace... "

    # Test 1: bf list
    if ! (cd "$workspace_path" && "$BF_BIN" list >/dev/null 2>&1); then
        echo "❌ FAIL (bf list crashed)"
        FAILED+=("$workspace (bf list)")
        continue
    fi

    # Test 2: bf sync --flush-only
    if ! (cd "$workspace_path" && "$BF_BIN" sync --flush-only >/dev/null 2>&1); then
        echo "❌ FAIL (bf sync --flush-only crashed)"
        FAILED+=("$workspace (bf sync)")
        continue
    fi

    echo "✅ PASS"
    PASSED+=("$workspace")
done

echo ""
echo "=== Results ==="
echo "Passed: ${#PASSED[@]}"
echo "Failed: ${#FAILED[@]}"

if [[ ${#FAILED[@]} -gt 0 ]]; then
    echo ""
    echo "Failed workspaces:"
    for failure in "${FAILED[@]}"; do
        echo "  - $failure"
    done
    exit 1
fi

echo ""
echo "All workspaces passed!"
exit 0
