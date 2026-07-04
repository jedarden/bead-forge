#!/bin/bash
# Test script for bf create command

set -e

TEST_DIR=$(mktemp -d)
export PATH="$HOME/.local/bin:$PATH"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BF_BINARY="$SCRIPT_DIR/target/debug/bf"

echo "Testing bf create in $TEST_DIR"

# Create test workspace
cd "$TEST_DIR"
mkdir -p .beads
cat > .beads/config.yaml <<EOF
issue_prefixes: [bf]
default_priority: 2
default_type: task
claim_ttl_minutes: 30
EOF

cat > .beads/metadata.json <<EOF
{"database": "beads.db", "jsonl_export": "issues.jsonl"}
EOF

echo "✓ Test workspace created"

# Initialize database
"$BF_BINARY" init --prefix bf
echo "✓ Database initialized"

# Test 1: Create minimal bead
echo ""
echo "Test 1: Create minimal bead"
BEAD1=$("$BF_BINARY" create --title "Test bead 1")
echo "Created bead: $BEAD1"

if [[ ! "$BEAD1" =~ ^bf-[a-z0-9]+$ ]]; then
    echo "✗ Invalid bead ID format: $BEAD1"
    exit 1
fi
echo "✓ Minimal bead created with valid ID"

# Test 2: Create bead with all parameters
echo ""
echo "Test 2: Create bead with all parameters"
BEAD2=$("$BF_BINARY" create \
    --title "Test bead with all params" \
    --type feature \
    --priority 1 \
    --description "This is a test description" \
    --assignee "test-worker" \
    --label "phase-1" \
    --label "test")
echo "Created bead: $BEAD2"
echo "✓ Bead created with all parameters"

# Test 3: Verify bead details
echo ""
echo "Test 3: Verify bead details"
"$BF_BINARY" show "$BEAD2" --format json > /tmp/bead2.json
echo "✓ Bead details retrieved"

# Parse and verify the JSON
TITLE=$(jq -r '.[0].title' /tmp/bead2.json)
TYPE=$(jq -r '.[0].issue_type' /tmp/bead2.json)
PRIORITY=$(jq -r '.[0].priority' /tmp/bead2.json)
DESCRIPTION=$(jq -r '.[0].description' /tmp/bead2.json)
ASSIGNEE=$(jq -r '.[0].assignee' /tmp/bead2.json)
LABELS=$(jq -r '.[0].labels | join(",")' /tmp/bead2.json)

if [ "$TITLE" != "Test bead with all params" ]; then
    echo "✗ Title mismatch: $TITLE"
    exit 1
fi

if [ "$TYPE" != "feature" ]; then
    echo "✗ Type mismatch: $TYPE"
    exit 1
fi

if [ "$PRIORITY" != "1" ]; then
    echo "✗ Priority mismatch: $PRIORITY"
    exit 1
fi

if [ "$DESCRIPTION" != "This is a test description" ]; then
    echo "✗ Description mismatch: $DESCRIPTION"
    exit 1
fi

if [ "$ASSIGNEE" != "test-worker" ]; then
    echo "✗ Assignee mismatch: $ASSIGNEE"
    exit 1
fi

echo "✓ All bead fields verified correctly"
echo "  - Title: $TITLE"
echo "  - Type: $TYPE"
echo "  - Priority: $PRIORITY"
echo "  - Description: $DESCRIPTION"
echo "  - Assignee: $ASSIGNEE"
echo "  - Labels: $LABELS"

# Test 4: Create beads with different types
echo ""
echo "Test 4: Create beads with different types"
for TYPE in task bug feature epic chore docs question; do
    BEAD=$("$BF_BINARY" create --title "Test $TYPE" --type "$TYPE")
    echo "✓ Created $TYPE: $BEAD"
done

# Test 5: List all beads
echo ""
echo "Test 5: List all beads"
BEAD_COUNT=$("$BF_BINARY" count)
echo "Total beads created: $BEAD_COUNT"
if [ "$BEAD_COUNT" -lt 9 ]; then
    echo "✗ Expected at least 9 beads, got $BEAD_COUNT"
    exit 1
fi
echo "✓ All beads accounted for"

# Test 6: Create bead with epic type
echo ""
echo "Test 6: Create epic bead"
EPIC_BEAD=$("$BF_BINARY" create --title "Test epic" --type epic)
echo "Created epic: $EPIC_BEAD"

# Verify epic type
"$BF_BINARY" show "$EPIC_BEAD" --format json > /tmp/epic.json
EPIC_TYPE=$(jq -r '.[0].issue_type' /tmp/epic.json)
if [ "$EPIC_TYPE" != "epic" ]; then
    echo "✗ Epic type mismatch: $EPIC_TYPE"
    exit 1
fi
echo "✓ Epic bead created successfully"

# Cleanup
cd -
rm -rf "$TEST_DIR"

echo ""
echo "=========================================="
echo "All tests passed! ✓"
echo "=========================================="
echo ""
echo "Summary:"
echo "- Minimal bead creation: ✓"
echo "- Full parameter bead creation: ✓"
echo "- Field verification: ✓"
echo "- Multiple issue types: ✓"
echo "- Bead counting: ✓"
echo "- Epic type creation: ✓"
