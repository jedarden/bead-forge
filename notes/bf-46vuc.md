# Duplicate Label Test Bead

## Task
Create a duplicate label test bead for testing purposes.

## What Was Done

Created duplicate bead `bf-42i8k`:
- Title: "Test label bead"
- Type: task
- Priority: P2
- Description: "Duplicated from bf-5pta4"
- Labels: duplicate,label,test
- Status: open

## Verification

```bash
$ br show bf-42i8k
ID: bf-42i8k
Title: Test label bead
Status: open
Priority: P2
Type: task
Description: Duplicated from bf-5pta4
Labels: duplicate,label,test
```

The duplicate bead was successfully created with the expected properties and labels.

## Results
- ✅ Bead created successfully
- ✅ Labels applied correctly (single label "duplicate,label,test")
- ✅ Description indicating source bead
- ✅ Status set to open
