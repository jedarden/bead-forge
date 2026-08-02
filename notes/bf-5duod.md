# Bead bf-5duod: Create Command Output Format

## Task
Implement create command output format to show only the bead ID.

## Implementation Status
**ALREADY IMPLEMENTED** - No code changes needed.

## Verification
The `cmd_create` function in `src/cli/mod.rs` (line 1596) already outputs just the bead ID:

```rust
} else {
    println!("{}", id);
}
```

## Test Results
Tested with actual `bf create` command:
- Input: `bf create --title "Test bead"`
- Output: `test-yo6\n` (just ID + newline)
- No debug text, no 'Created:' prefix, no extra whitespace

## Byte-level Verification
```
$ bf create --title "Verify format" | od -c
0000000   t   e   s   t   -   2   7   v  \n
0000011
```

The output matches br's exact format for compatibility.
