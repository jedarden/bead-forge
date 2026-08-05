//! Three-way JSONL merge for bead-forge (Phase 7.9).
//!
//! Multi-box fleets keep the live state in per-checkout SQLite databases and
//! share it through the git-committed `issues.jsonl` artifact. When two
//! checkouts diverge (the recurring lab/ex44 case), a plain git text merge of
//! `issues.jsonl` is hazardous: the file is line-oriented but each line is a
//! whole bead, so a textual conflict marker corrupts JSON and a "take theirs"
//! silently drops beads created on the other box.
//!
//! This module performs an id-keyed three-way merge instead. Given a common
//! ancestor snapshot (`beads.base.jsonl`, the *merge anchor*) plus the two
//! divergent versions, it resolves every bead independently:
//!
//! * only one side changed a bead  -> take the changed side
//! * both sides made the *same*    -> take it, no conflict
//!   change
//! * both sides changed a bead     -> deterministic last-writer-wins by
//!   differently                      `updated_at` (ties broken by content hash)
//! * one side deleted, other kept  -> keep the surviving/modified bead so a
//!   or modified                      concurrent edit is never silently lost
//!
//! The result is written sorted by id for stable diffs. The same entry point
//! doubles as a git merge driver (see `bf merge-jsonl --help`).

use crate::model::Issue;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Filename (under `.beads/`) of the merge anchor: the last-synced common
/// ancestor snapshot used as the base for three-way merges across checkouts.
pub const BASE_ANCHOR: &str = "beads.base.jsonl";

/// Path to the merge anchor for a given `.beads` directory.
pub fn base_anchor_path(beads_dir: &Path) -> PathBuf {
    beads_dir.join(BASE_ANCHOR)
}

/// Update the merge anchor to mirror the current `issues.jsonl`.
///
/// Called after a full flush or import so the anchor always reflects the state
/// this box last agreed on with the git artifact. The next divergent merge
/// uses it as the three-way base. Best-effort: a missing source is a no-op.
pub fn update_base_anchor(beads_dir: &Path, jsonl_path: &Path) -> Result<()> {
    if !jsonl_path.exists() {
        return Ok(());
    }
    let anchor = base_anchor_path(beads_dir);
    let temp = anchor.with_extension("base.jsonl.tmp");
    std::fs::copy(jsonl_path, &temp)
        .with_context(|| format!("copying anchor from {}", jsonl_path.display()))?;
    std::fs::rename(&temp, &anchor)
        .with_context(|| format!("installing anchor {}", anchor.display()))?;
    Ok(())
}

/// Statistics describing the outcome of a three-way merge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// Beads present in the merged output that did not exist in the base.
    pub added: usize,
    /// Beads that existed in the base and were changed by exactly one side.
    pub updated: usize,
    /// Beads that existed in the base and are absent from the merged output.
    pub deleted: usize,
    /// Beads where both sides diverged and a deterministic winner was chosen,
    /// or a delete raced a modify. These are auto-resolved, not left as markers.
    pub conflicts: usize,
    /// Beads carried through untouched.
    pub unchanged: usize,
    /// Total beads in the merged output.
    pub total: usize,
}

impl MergeReport {
    /// Whether any bead required conflict resolution.
    #[must_use]
    pub fn had_conflicts(&self) -> bool {
        self.conflicts > 0
    }
}

/// Parse a JSONL file into an id-keyed map.
///
/// A missing file is treated as an empty set (a fresh checkout that has never
/// flushed, or a merge base that predates the artifact). Malformed lines are a
/// hard error — we refuse to guess when the artifact is corrupt.
pub fn load_map(path: &Path) -> Result<BTreeMap<String, Issue>> {
    let mut map = BTreeMap::new();
    if !path.exists() {
        return Ok(map);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading JSONL {}", path.display()))?;
    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let issue: Issue = serde_json::from_str(line)
            .with_context(|| format!("parsing {} line {}", path.display(), lineno + 1))?;
        map.insert(issue.id.clone(), issue);
    }
    Ok(map)
}

/// Return true if `a` and `b` represent the same synced bead payload.
fn same(a: &Issue, b: &Issue) -> bool {
    a.sync_equals(b)
}

/// Choose a deterministic winner between two diverged versions of a bead.
///
/// Last-writer-wins by `updated_at`; ties broken by content hash so the result
/// does not depend on which checkout runs the merge or argument order.
fn resolve_conflict(ours: Issue, theirs: Issue) -> Issue {
    match ours.updated_at.cmp(&theirs.updated_at) {
        std::cmp::Ordering::Greater => ours,
        std::cmp::Ordering::Less => theirs,
        std::cmp::Ordering::Equal => {
            if ours.content_hash() >= theirs.content_hash() {
                ours
            } else {
                theirs
            }
        }
    }
}

/// Perform a three-way, id-keyed merge of bead maps.
///
/// Returns the merged beads sorted by id plus a [`MergeReport`]. Never emits
/// conflict markers — every divergence is auto-resolved deterministically.
pub fn merge_maps(
    base: &BTreeMap<String, Issue>,
    ours: &BTreeMap<String, Issue>,
    theirs: &BTreeMap<String, Issue>,
) -> (Vec<Issue>, MergeReport) {
    // Union of all ids across the three inputs (BTreeMap keeps this sorted).
    let mut ids: BTreeMap<&String, ()> = BTreeMap::new();
    for k in base.keys().chain(ours.keys()).chain(theirs.keys()) {
        ids.insert(k, ());
    }

    let mut out: Vec<Issue> = Vec::new();
    let mut report = MergeReport::default();

    for id in ids.keys() {
        let b = base.get(*id);
        let o = ours.get(*id);
        let t = theirs.get(*id);

        let chosen: Option<(Issue, Decision)> = match (o, t) {
            // Present on both sides.
            (Some(o), Some(t)) => {
                if same(o, t) {
                    // Identical on both sides: no conflict regardless of base.
                    let decision = classify_present(b, o);
                    Some((o.clone(), decision))
                } else {
                    let o_changed = b.map_or(true, |bb| !same(bb, o));
                    let t_changed = b.map_or(true, |bb| !same(bb, t));
                    match (o_changed, t_changed) {
                        (true, false) => Some((o.clone(), classify_present(b, o))),
                        (false, true) => Some((t.clone(), classify_present(b, t))),
                        // Both changed (or a base-less double-add that differs):
                        // deterministic winner, flagged as a conflict.
                        _ => {
                            let winner = resolve_conflict(o.clone(), t.clone());
                            Some((
                                winner,
                                Decision::Conflict {
                                    existed: b.is_some(),
                                },
                            ))
                        }
                    }
                }
            }
            // Present only on ours; theirs dropped it (or never had it).
            (Some(o), None) => match b {
                None => Some((o.clone(), Decision::Added)),
                Some(bb) => {
                    if same(bb, o) {
                        // Unchanged on ours, deleted on theirs -> honor delete.
                        None
                    } else {
                        // Modified on ours, deleted on theirs -> keep the edit.
                        report.conflicts += 1;
                        Some((o.clone(), Decision::Kept))
                    }
                }
            },
            // Present only on theirs; ours dropped it.
            (None, Some(t)) => match b {
                None => Some((t.clone(), Decision::Added)),
                Some(bb) => {
                    if same(bb, t) {
                        None
                    } else {
                        report.conflicts += 1;
                        Some((t.clone(), Decision::Kept))
                    }
                }
            },
            // Deleted on both sides.
            (None, None) => None,
        };

        match chosen {
            Some((issue, decision)) => {
                match decision {
                    Decision::Added => report.added += 1,
                    Decision::Updated => report.updated += 1,
                    Decision::Unchanged => report.unchanged += 1,
                    Decision::Kept => {} // conflict already counted
                    Decision::Conflict { existed } => {
                        report.conflicts += 1;
                        if existed {
                            report.updated += 1;
                        } else {
                            report.added += 1;
                        }
                    }
                }
                out.push(issue);
            }
            None => {
                if b.is_some() {
                    report.deleted += 1;
                }
            }
        }
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    report.total = out.len();
    (out, report)
}

/// Internal decision tag used only to drive report counters.
enum Decision {
    Added,
    Updated,
    Unchanged,
    /// Kept a modified bead over a concurrent delete (conflict counted inline).
    Kept,
    Conflict {
        existed: bool,
    },
}

/// Classify a bead that survives to the output relative to the base.
fn classify_present(base: Option<&Issue>, chosen: &Issue) -> Decision {
    match base {
        None => Decision::Added,
        Some(b) => {
            if same(b, chosen) {
                Decision::Unchanged
            } else {
                Decision::Updated
            }
        }
    }
}

/// Write merged issues to a path atomically (temp file + rename), sorted by id.
pub fn write_jsonl(path: &Path, issues: &[Issue]) -> Result<()> {
    let temp_path = path.with_extension("jsonl.merge.tmp");
    {
        let file = std::fs::File::create(&temp_path)
            .with_context(|| format!("creating {}", temp_path.display()))?;
        let mut writer = BufWriter::new(file);
        for issue in issues {
            serde_json::to_writer(&mut writer, issue)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
    }
    std::fs::rename(&temp_path, path)
        .with_context(|| format!("renaming into {}", path.display()))?;
    Ok(())
}

/// Merge three JSONL files and write the result to `output`.
///
/// Designed to double as a git merge driver: point `output` at the "ours"
/// (`%A`) path and git will pick up the resolved artifact. A missing `base`
/// file degrades gracefully to a two-way union (every difference becomes an
/// add/conflict rather than a delete), which is the safe direction.
pub fn merge_jsonl_files(
    base: &Path,
    ours: &Path,
    theirs: &Path,
    output: &Path,
) -> Result<MergeReport> {
    let base_map = load_map(base).with_context(|| "loading merge base")?;
    let ours_map = load_map(ours).with_context(|| "loading ours")?;
    let theirs_map = load_map(theirs).with_context(|| "loading theirs")?;

    let (merged, report) = merge_maps(&base_map, &ours_map, &theirs_map);
    write_jsonl(output, &merged)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{IssueType, Priority, Status};
    use chrono::{Duration, Utc};

    fn issue(id: &str, title: &str, updated_offset_secs: i64) -> Issue {
        let base = Utc::now();
        Issue {
            id: id.to_string(),
            title: title.to_string(),
            status: Status::Open,
            priority: Priority::MEDIUM,
            issue_type: IssueType::Task,
            created_at: base,
            updated_at: base + Duration::seconds(updated_offset_secs),
            source_repo: Some(".".to_string()),
            ..Default::default()
        }
    }

    fn to_map(issues: Vec<Issue>) -> BTreeMap<String, Issue> {
        issues.into_iter().map(|i| (i.id.clone(), i)).collect()
    }

    #[test]
    fn merge_disjoint_adds_from_both_sides() {
        // Classic lab/ex44 divergence: each box created its own beads.
        let base = to_map(vec![issue("bf-1", "shared", 0)]);
        let ours = to_map(vec![
            issue("bf-1", "shared", 0),
            issue("bf-2", "ours-only", 0),
        ]);
        let theirs = to_map(vec![
            issue("bf-1", "shared", 0),
            issue("bf-3", "theirs-only", 0),
        ]);

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        let ids: Vec<&str> = merged.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["bf-1", "bf-2", "bf-3"], "no bead may be dropped");
        assert_eq!(report.added, 2);
        assert_eq!(report.unchanged, 1);
        assert_eq!(report.conflicts, 0);
        assert_eq!(report.total, 3);
    }

    #[test]
    fn merge_one_sided_change_takes_that_side() {
        let base = to_map(vec![issue("bf-1", "original", 0)]);
        let ours = to_map(vec![issue("bf-1", "edited-by-us", 10)]);
        let theirs = to_map(vec![issue("bf-1", "original", 0)]);

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "edited-by-us");
        assert_eq!(report.updated, 1);
        assert_eq!(report.conflicts, 0);
    }

    #[test]
    fn merge_both_changed_last_writer_wins() {
        let base = to_map(vec![issue("bf-1", "original", 0)]);
        let ours = to_map(vec![issue("bf-1", "ours", 5)]);
        let theirs = to_map(vec![issue("bf-1", "theirs", 20)]); // later updated_at

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "theirs", "later updated_at wins");
        assert_eq!(report.conflicts, 1);
        assert_eq!(report.updated, 1);
    }

    #[test]
    fn merge_is_order_independent() {
        let base = to_map(vec![issue("bf-1", "original", 0)]);
        let ours = to_map(vec![issue("bf-1", "ours", 5)]);
        let theirs = to_map(vec![issue("bf-1", "theirs", 20)]);

        let (ab, _) = merge_maps(&base, &ours, &theirs);
        let (ba, _) = merge_maps(&base, &theirs, &ours);
        assert_eq!(
            ab[0].title, ba[0].title,
            "winner must not depend on side order"
        );
    }

    #[test]
    fn merge_same_edit_on_both_sides_no_conflict() {
        let base = to_map(vec![issue("bf-1", "original", 0)]);
        let ours = to_map(vec![issue("bf-1", "converged", 10)]);
        let theirs = to_map(vec![issue("bf-1", "converged", 10)]);

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].title, "converged");
        assert_eq!(report.conflicts, 0);
        assert_eq!(report.updated, 1);
    }

    #[test]
    fn merge_delete_unchanged_other_side_deletes() {
        let base = to_map(vec![issue("bf-1", "doomed", 0), issue("bf-2", "keep", 0)]);
        let ours = to_map(vec![issue("bf-2", "keep", 0)]); // we deleted bf-1
        let theirs = to_map(vec![issue("bf-1", "doomed", 0), issue("bf-2", "keep", 0)]);

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        let ids: Vec<&str> = merged.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["bf-2"]);
        assert_eq!(report.deleted, 1);
        assert_eq!(report.conflicts, 0);
    }

    #[test]
    fn merge_delete_vs_modify_keeps_modification() {
        // Data-loss guard: a concurrent edit must survive a delete on the other box.
        let base = to_map(vec![issue("bf-1", "original", 0)]);
        let ours = to_map(vec![]); // we deleted bf-1
        let theirs = to_map(vec![issue("bf-1", "still-being-worked", 30)]); // they edited it

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        assert_eq!(merged.len(), 1, "modification must win over delete");
        assert_eq!(merged[0].title, "still-being-worked");
        assert_eq!(report.conflicts, 1);
    }

    #[test]
    fn merge_missing_base_degrades_to_union() {
        let base = BTreeMap::new();
        let ours = to_map(vec![issue("bf-1", "ours", 0)]);
        let theirs = to_map(vec![issue("bf-2", "theirs", 0)]);

        let (merged, report) = merge_maps(&base, &ours, &theirs);
        assert_eq!(merged.len(), 2, "no base -> safe union, nothing deleted");
        assert_eq!(report.deleted, 0);
        assert_eq!(report.added, 2);
    }

    #[test]
    fn merge_files_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let base_p = dir.path().join("base.jsonl");
        let ours_p = dir.path().join("ours.jsonl");
        let theirs_p = dir.path().join("theirs.jsonl");

        write_jsonl(&base_p, &[issue("bf-1", "shared", 0)]).unwrap();
        write_jsonl(
            &ours_p,
            &[issue("bf-1", "shared", 0), issue("bf-2", "ours", 0)],
        )
        .unwrap();
        write_jsonl(
            &theirs_p,
            &[issue("bf-1", "shared", 0), issue("bf-3", "theirs", 0)],
        )
        .unwrap();

        // git merge driver style: write result back over "ours".
        let report = merge_jsonl_files(&base_p, &ours_p, &theirs_p, &ours_p).unwrap();
        assert_eq!(report.total, 3);

        let reread = load_map(&ours_p).unwrap();
        assert!(reread.contains_key("bf-1"));
        assert!(reread.contains_key("bf-2"));
        assert!(reread.contains_key("bf-3"));
    }
}
