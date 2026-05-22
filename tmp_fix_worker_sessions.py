#!/usr/bin/env python3
import sqlite3

conn = sqlite3.connect('.beads/beads.db')
cursor = conn.cursor()

# Check worker_sessions for bf-1leo
cursor.execute("SELECT bead_id, claimed_at, closed_at, duration_seconds FROM worker_sessions WHERE bead_id = 'bf-1leo'")
rows = cursor.fetchall()

print("Worker sessions for bf-1leo:")
for row in rows:
    print(f"  bead_id={row[0]}, claimed_at={row[1]}, closed_at={row[2]}, duration={row[3]}")

# Delete malformed entries (empty claimed_at)
cursor.execute("DELETE FROM worker_sessions WHERE bead_id = 'bf-1leo' AND (claimed_at IS NULL OR claimed_at = '')")
affected = cursor.rowcount
print(f"\nDeleted {affected} malformed worker_sessions entries")

conn.commit()
conn.close()
