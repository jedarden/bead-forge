# bf-1w20y: Add checksum verification to bf-update.sh before installing downloaded binary

## Status: COMPLETE (Already Implemented)

This bead was fully implemented in July 2026 but was never closed. The implementation exists in commits:
- `2d388ad` (2026-07-21): Initial SHA256 checksum verification
- `aacb597` (2026-07-22): Security fix - match checksum by filename

## Implementation Summary

### Client-side (bf-update.sh)

Both `deploy/bf-update.sh` and `scripts/bf-update.sh` now:

1. **Download SHA256SUMS alongside binary** (lines 55-70):
   - Fetch release manifest once
   - Extract both `bf-linux-x86_64` and `SHA256SUMS` asset URLs
   - Download both to temp directory

2. **Verify downloads succeeded** (lines 72-80):
   - Check binary exists
   - Check SHA256SUMS exists
   - Fail loudly if either is missing

3. **Verify checksum BEFORE installing** (lines 82-106):
   - Extract expected hash for `bf-linux-x86_64` specifically (security: prevents attacker from listing different file first)
   - Compute actual SHA256 of downloaded binary
   - Compare and refuse installation on mismatch
   - Support both text (`<hash>  name`) and binary (`<hash> *name`) manifest formats
   - Fallback for legacy bare-hash manifests

4. **Fail safely**:
   - Non-zero exit on any failure
   - Clear error messages
   - Leave existing `bf` binary untouched
   - No silent failures

### CI-side (bead-forge-build workflow)

The WorkflowTemplate in `declarative-config` (lines 48-64) now:

1. **Generates SHA256SUMS**:
   ```bash
   ( cd ./target/release && sha256sum bf-linux-x86_64 > SHA256SUMS )
   ```

2. **Publishes both assets to GitHub Releases**:
   ```bash
   gh release create "v${VERSION}" \
     "./target/release/bf-linux-x86_64" \
     "./target/release/SHA256SUMS"
   ```

## Verification

Both scripts are functionally equivalent for checksum verification:
- `deploy/bf-update.sh` - Debian/Ubuntu variant (also has GitHub API auth for rate limiting)
- `scripts/bf-update.sh` - NixOS variant (minimal, no auth)

All security requirements met:
- ✅ SHA256SUMS downloaded and verified
- ✅ Checksum matches expected hash for specific binary
- ✅ Fail loudly on mismatch
- ✅ Leave old binary in place on failure
- ✅ CI publishes SHA256SUMS with each release

## Files Modified

- `deploy/bf-update.sh` - Full implementation + GitHub auth
- `scripts/bf-update.sh` - Full implementation (no auth)
- `docs/README.md` - Documents SHA256SUMS requirement
- `deploy/README.md` - Updated "How it works" section
- `systemd/README.md` - Updated "How it works" section
- `declarative-config/k8s/iad-ci/argo-workflows/bead-forge-build-workflowtemplate.yml` - Generates and uploads SHA256SUMS
