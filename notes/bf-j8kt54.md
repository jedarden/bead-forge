# Execute single test module with output capture — bf-j8kt54

Second step in the test-execution chain: validate the trace-capture mechanism
with a smaller scope before running the full suite. Builds on the setup done in
bf-1yxdgq (capture infrastructure) and bf-1h3ug0 (prerequisite verification).

## Command run

The exact capture syntax documented in `notes/bf-1yxdgq.md` (combined
stdout+stderr to a single file):

```bash
cargo test --lib id > .beads/traces/bf-j8kt54/test-run.log 2>&1
```

This is the documented single-module form (`cargo test --lib <module>`).
`--lib` restricts the run to the library test target only (no integration
`tests/*.rs` binaries), and the `id` filter scopes it to a subset — 532 of the
629 lib tests were filtered out, so this is clearly **not** the full suite.

## Acceptance criteria — all ✅

| AC | Result |
|----|--------|
| Execute cargo test for a single test module (not full suite) | ✅ 97 of 629 lib tests ran; 532 filtered out |
| Redirect both stdout and stderr using bf-1yxdgq syntax | ✅ `> ... 2>&1` merge; nothing leaked to terminal |
| Verify trace file created and contains output | ✅ `.beads/traces/bf-j8kt54/test-run.log`, 7574 bytes / 102 lines |
| Confirm test execution completes (even if tests fail) | ✅ exit 0 — `97 passed; 0 failed; 0 ignored` in 1.75s |

## Result

```
test result: ok. 97 passed; 0 failed; 0 ignored; 0 measured; 532 filtered out; finished in 1.75s
```

The capture mechanism is validated: the log contains the test-harness output
(`running 97 tests`, per-test `... ok` lines, and the final `test result:` line),
the file is non-empty, and the run completed cleanly with exit code 0.

## Artifacts

- `.beads/traces/bf-j8kt54/metadata.json` — trace metadata (exit_code=0,
  outcome=success, test counts)
- `.beads/traces/bf-j8kt54/test-run.log` — captured combined stdout+stderr

Environment is confirmed ready to run the full test suite with the same
capture syntax.
