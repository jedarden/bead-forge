#!/bin/bash
# Manual verification script for multi-label P0 priority bead bf-55ma7u
# This script verifies that the bead has the expected properties and that operations work

set -e

echo "=========================================="
echo "P0 Multi-label Bead Verification"
echo "=========================================="
echo ""

BEAD_ID="bf-55ma7u"

echo "1. Verify bead exists and has P0 priority"
echo "------------------------------------------"
bf show "$BEAD_ID" --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)[0]
assert data['priority'] == 0, 'Priority must be P0 (0)'
print('✓ Priority is P0')
print(f'  Priority value: {data[\"priority\"]}')
"

echo ""
echo "2. Verify bead has multiple labels"
echo "------------------------------------------"
bf show "$BEAD_ID" --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)[0]
labels = data['labels']
assert len(labels) > 1, 'Must have multiple labels'
print(f'✓ Has {len(labels)} labels')
print(f'  Labels: {\", \".join(labels)}')
"

echo ""
echo "3. Verify specific labels exist"
echo "------------------------------------------"
bf show "$BEAD_ID" --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)[0]
labels = data['labels']
required_labels = ['multi-label', 'p0-test']
for label in required_labels:
    assert label in labels, f'Missing required label: {label}'
    print(f'✓ Has label: {label}')
"

echo ""
echo "4. Test JSON serialization"
echo "------------------------------------------"
OUTPUT=$(bf show "$BEAD_ID" --format json)
echo "$OUTPUT" | python3 -c "
import json, sys
data = json.load(sys.stdin)[0]
json_str = json.dumps(data)
assert '\"priority\":0' in json_str, 'Priority must be serialized as 0'
assert '\"labels\":[' in json_str, 'Labels array must be present'
print('✓ JSON serialization correct')
print('  Priority serialized as 0')
print('  Labels array present')
"

echo ""
echo "5. Verify status is in_progress"
echo "------------------------------------------"
bf show "$BEAD_ID" --format json | python3 -c "
import json, sys
data = json.load(sys.stdin)[0]
assert data['status'] == 'in_progress', 'Status must be in_progress'
print('✓ Status is in_progress')
"

echo ""
echo "6. List all beads to verify visibility"
echo "------------------------------------------"
bf list | grep -q "$BEAD_ID" && echo "✓ Bead visible in list" || echo "✗ Bead not found in list"

echo ""
echo "=========================================="
echo "All Verifications Passed!"
echo "=========================================="
echo ""
echo "Summary:"
echo "  - Bead ID: $BEAD_ID"
echo "  - Priority: P0 (critical)"
echo "  - Has multiple labels: YES"
echo "  - Labels include: multi-label, p0-test"
echo "  - JSON serialization: WORKING"
echo "  - Status: in_progress"
echo ""
