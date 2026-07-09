# Epic P0 Creation Test (bf-63bl8)

## Test Summary
Tested the creation of an epic issue with critical (P0) priority using the br CLI.

## Test Command
```bash
br create --title "Test Epic P0 Creation" --type epic --priority 0 --description "Testing epic creation with critical priority"
```

## Result
✓ Successfully created epic with ID `bf-1on21`

## Verification
```bash
br show bf-1on21
```

Output:
```
ID: bf-1on21
Title: Test Epic P0 Creation
Status: open
Priority: P0
Type: epic
Description: Testing epic creation with critical priority
```

## Conclusion
Epic creation with P0 priority works correctly. The `br create` command properly accepts:
- `--type epic` for epic type
- `--priority 0` for critical priority

Both fields are correctly stored and displayed in the bead details.
