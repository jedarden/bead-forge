# Coverage Tooling Setup - bf-3hf64

## Task Completed

Set up coverage tooling and generated baseline coverage report for bead-forge.

## Configuration

**Tool**: cargo-tarpaulin 0.31 (already configured in `Cargo.toml` dev-dependencies)

**Config File**: `.tarpaulin.toml`
- Output format: HTML
- Output directory: `.tarpaulin/`
- Timeout: 60s
- Runs tests only (not doctests, benchmarks, etc.)

**NixOS Integration**: Run coverage via nix-shell to provide OpenSSL dependencies:
```bash
nix-shell -p openssl pkg-config --run "cargo tarpaulin --out Html --output-dir .tarpaulin"
```

## Baseline Report

**Location**: `.tarpaulin/html/index.html`

**Overall Coverage**: 77.43% line coverage (11,439/14,773 lines)

**Generated**: 2026-07-23 13:06

### Readonly Command Coverage

All readonly commands have coverage data:

| Command | Module | Line Coverage |
|---------|--------|---------------|
| list, show, ready, status, labels, comments | cli/mod.rs | 65.41% |
| critical-path | critical_path.rs | 94.70% |
| doctor | doctor.rs | 89.77% |
| sync --status | sync.rs | 88.41% |
| velocity | velocity.rs | 77.40% |
| commit_check | commit_check.rs | 70.53% |

## Viewing the Report

Open locally in a browser:
```bash
python3 -m http.server 8000 --directory .tarpaulin/html
# Then visit http://localhost:8000/index.html
```

## CI Integration

For future CI runs, add tarpaulin step (requires nix-shell or pre-installed OpenSSL):
```yaml
- name: Generate coverage
  run: nix-shell -p openssl pkg-config --run "cargo tarpaulin --out Html --output-dir .tarpaulin"
```
