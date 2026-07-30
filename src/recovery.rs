//! Verified pre-rebuild backups for the doctor safety stack (Phase 7.2, layer 3).
//!
//! Before the doctor ever rebuilds SQLite from JSONL (a last-resort, destructive
//! operation), it snapshots the full DB family plus the JSONL authority into a
//! per-run recovery directory and records a SHA-256 hash for every copied file.
//! Because each backup carries verifiable hashes:
//!
//! * a rebuild that fails post-verification can be rolled back to a known-good state
//!   automatically (`restore_run`), and
//! * an operator can list past runs (`bf doctor --runs`) and restore any of them by
//!   id or `latest` (`bf doctor --restore <run-id|latest>`).
//!
//! The recovery directory lives at `.beads/recovery/`. Each run is a subdirectory
//! named for its run id (a filesystem-safe UTC timestamp) containing byte-for-byte
//! copies of the backed-up files and a `manifest.json` describing them.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Name of the recovery directory under `.beads/`.
pub const RECOVERY_DIR: &str = "recovery";

/// Name of the per-run manifest file.
const MANIFEST_NAME: &str = "manifest.json";

/// Marker written when a rebuild fails its post-verification (layer 5). While it
/// exists, further rebuilds refuse unless `--allow-repeated-repair` is passed.
const REPAIR_FAILED_MARKER: &str = "repair-failed.marker";

/// One backed-up file and its verification hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackedUpFile {
    /// File name as it appears in `.beads/` (e.g. `beads.db`, `beads.db-wal`).
    pub name: String,
    /// Lowercase hex SHA-256 of the file's bytes at backup time.
    pub sha256: String,
    /// Size in bytes at backup time.
    pub bytes: u64,
}

/// Describes a single recovery run: what was backed up, when, and why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    /// Filesystem-safe run identifier (also the subdirectory name).
    pub run_id: String,
    /// RFC3339 creation timestamp.
    pub created_at: String,
    /// Human-readable reason (e.g. "pre-rebuild").
    pub reason: String,
    /// Files captured in this run, each with a verification hash.
    pub files: Vec<BackedUpFile>,
}

/// Compute the lowercase hex SHA-256 of a file's contents.
pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read {} for hashing", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{:x}", digest))
}

/// Path to the recovery directory for a given `.beads` directory.
pub fn recovery_dir(beads_dir: &Path) -> PathBuf {
    beads_dir.join(RECOVERY_DIR)
}

/// Generate a filesystem-safe run id from the current time.
///
/// Millisecond resolution keeps two rebuilds in the same second from colliding.
fn generate_run_id() -> String {
    let now = chrono::Utc::now();
    format!(
        "run-{}-{:03}",
        now.format("%Y%m%dT%H%M%S"),
        now.timestamp_subsec_millis()
    )
}

/// Create a verified backup of `files` into a fresh run directory.
///
/// Each source path that exists is copied into `.beads/recovery/<run-id>/` under
/// its file name, and its SHA-256 hash is recorded in the run manifest. Sources
/// that do not exist (e.g. a missing `-wal`/`-shm` sidecar) are skipped silently.
///
/// Returns the manifest describing the run.
pub fn create_backup(beads_dir: &Path, files: &[PathBuf], reason: &str) -> Result<BackupManifest> {
    let root = recovery_dir(beads_dir);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create recovery dir {}", root.display()))?;

    let run_id = generate_run_id();
    let run_dir = root.join(&run_id);
    std::fs::create_dir_all(&run_dir)
        .with_context(|| format!("Failed to create run dir {}", run_dir.display()))?;

    let mut backed_up = Vec::new();
    for src in files {
        if !src.exists() {
            continue;
        }
        let name = src
            .file_name()
            .ok_or_else(|| anyhow!("Backup source has no file name: {}", src.display()))?
            .to_string_lossy()
            .into_owned();
        let dest = run_dir.join(&name);
        std::fs::copy(src, &dest)
            .with_context(|| format!("Failed to copy {} to backup", src.display()))?;

        // Hash the copy we just wrote; if the copy is faithful its hash matches the
        // source, and this is the exact byte stream a later restore reads back.
        let sha256 = hash_file(&dest)?;
        let bytes = std::fs::metadata(&dest)?.len();
        backed_up.push(BackedUpFile {
            name,
            sha256,
            bytes,
        });
    }

    let manifest = BackupManifest {
        run_id: run_id.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        reason: reason.to_string(),
        files: backed_up,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(run_dir.join(MANIFEST_NAME), manifest_json)
        .with_context(|| format!("Failed to write manifest for run {}", run_id))?;

    Ok(manifest)
}

/// Load the manifest for a single run directory.
fn load_manifest(run_dir: &Path) -> Result<BackupManifest> {
    let manifest_path = run_dir.join(MANIFEST_NAME);
    let json = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest {}", manifest_path.display()))?;
    let manifest: BackupManifest = serde_json::from_str(&json)
        .with_context(|| format!("Failed to parse manifest {}", manifest_path.display()))?;
    Ok(manifest)
}

/// List all recovery runs, newest first.
///
/// Run ids are lexicographically ordered by construction (timestamp prefix), so a
/// reverse sort yields newest-first without parsing timestamps.
pub fn list_runs(beads_dir: &Path) -> Result<Vec<BackupManifest>> {
    let root = recovery_dir(beads_dir);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut runs = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let run_dir = entry.path();
        if !run_dir.join(MANIFEST_NAME).exists() {
            continue;
        }
        match load_manifest(&run_dir) {
            Ok(m) => runs.push(m),
            Err(_) => continue, // skip unreadable/partial runs rather than failing the listing
        }
    }

    runs.sort_by(|a, b| b.run_id.cmp(&a.run_id));
    Ok(runs)
}

/// Resolve a run reference ("latest" or an explicit run id) to a manifest.
pub fn resolve_run(beads_dir: &Path, run_ref: &str) -> Result<BackupManifest> {
    if run_ref == "latest" {
        return list_runs(beads_dir)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No recovery runs found to restore"));
    }
    let run_dir = recovery_dir(beads_dir).join(run_ref);
    if !run_dir.exists() {
        return Err(anyhow!("Recovery run '{}' not found", run_ref));
    }
    load_manifest(&run_dir)
}

/// Verify that every file in a run still matches its recorded hash.
///
/// Returns `Ok(())` when all hashes match; otherwise an error naming the first
/// file that failed verification (a corrupt or truncated backup must never be
/// silently restored over a live database).
pub fn verify_run(beads_dir: &Path, run_id: &str) -> Result<()> {
    let run_dir = recovery_dir(beads_dir).join(run_id);
    let manifest = load_manifest(&run_dir)?;
    for f in &manifest.files {
        let path = run_dir.join(&f.name);
        if !path.exists() {
            return Err(anyhow!(
                "Backup file {} missing from run {}",
                f.name,
                run_id
            ));
        }
        let actual = hash_file(&path)?;
        if actual != f.sha256 {
            return Err(anyhow!(
                "Backup file {} in run {} failed hash verification (expected {}, got {})",
                f.name,
                run_id,
                f.sha256,
                actual
            ));
        }
    }
    Ok(())
}

/// Restore a run's files back into `.beads/`, after verifying every hash.
///
/// The DB family (`beads.db` and any `-wal`/`-shm` sidecars) present in the backup
/// replaces the live copies. To avoid a stale sidecar from the *current* (failed)
/// state resurrecting after restore, any live `-wal`/`-shm` not captured in the
/// backup is removed first.
pub fn restore_run(beads_dir: &Path, run_ref: &str) -> Result<BackupManifest> {
    let manifest = resolve_run(beads_dir, run_ref)?;
    verify_run(beads_dir, &manifest.run_id)?;

    let run_dir = recovery_dir(beads_dir).join(&manifest.run_id);
    let backed_up_names: std::collections::HashSet<&str> =
        manifest.files.iter().map(|f| f.name.as_str()).collect();

    // Clear live SQLite sidecars that are not part of the backup; leaving a stale
    // WAL/SHM alongside a restored .db can corrupt reads.
    for sidecar in ["-wal", "-shm"] {
        for f in &manifest.files {
            if f.name.ends_with(".db") {
                let live_sidecar = beads_dir.join(format!("{}{}", f.name, sidecar));
                let sidecar_name = format!("{}{}", f.name, sidecar);
                if live_sidecar.exists() && !backed_up_names.contains(sidecar_name.as_str()) {
                    let _ = std::fs::remove_file(&live_sidecar);
                }
            }
        }
    }

    for f in &manifest.files {
        let src = run_dir.join(&f.name);
        let dest = beads_dir.join(&f.name);
        std::fs::copy(&src, &dest)
            .with_context(|| format!("Failed to restore {} from run {}", f.name, manifest.run_id))?;
    }

    Ok(manifest)
}

// ---- Repeat-failure marker (layer 5) ----

/// Path to the repeat-failure marker file.
fn marker_path(beads_dir: &Path) -> PathBuf {
    recovery_dir(beads_dir).join(REPAIR_FAILED_MARKER)
}

/// Whether a prior rebuild left a repeat-failure marker.
pub fn repair_failed_marker_exists(beads_dir: &Path) -> bool {
    marker_path(beads_dir).exists()
}

/// Write the repeat-failure marker, recording which run's backup was restored.
pub fn write_repair_failed_marker(beads_dir: &Path, detail: &str) -> Result<()> {
    let root = recovery_dir(beads_dir);
    std::fs::create_dir_all(&root)?;
    let body = format!(
        "rebuild post-verification failed at {}\n{}\n",
        chrono::Utc::now().to_rfc3339(),
        detail
    );
    std::fs::write(marker_path(beads_dir), body)?;
    Ok(())
}

/// Remove the repeat-failure marker (called after a successful rebuild).
pub fn clear_repair_failed_marker(beads_dir: &Path) -> Result<()> {
    let path = marker_path(beads_dir);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(path: &Path, contents: &[u8]) {
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn backup_records_hashes_and_verifies() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        let db = beads.join("beads.db");
        let jsonl = beads.join("issues.jsonl");
        write(&db, b"database-bytes");
        write(&jsonl, b"{\"id\":\"bf-1\"}\n");

        let manifest =
            create_backup(&beads, &[db.clone(), jsonl.clone()], "pre-rebuild").unwrap();
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.reason, "pre-rebuild");
        // Hash matches an independent computation of the source bytes.
        let db_entry = manifest.files.iter().find(|f| f.name == "beads.db").unwrap();
        assert_eq!(db_entry.sha256, hash_file(&db).unwrap());

        // A fresh backup verifies cleanly.
        verify_run(&beads, &manifest.run_id).unwrap();
    }

    #[test]
    fn missing_sidecar_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        let db = beads.join("beads.db");
        write(&db, b"db");
        let missing_wal = beads.join("beads.db-wal");

        let manifest = create_backup(&beads, &[db, missing_wal], "pre-rebuild").unwrap();
        // Only the file that existed is captured.
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].name, "beads.db");
    }

    #[test]
    fn verify_detects_tampering() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        let db = beads.join("beads.db");
        write(&db, b"original");
        let manifest = create_backup(&beads, &[db], "pre-rebuild").unwrap();

        // Tamper with the backed-up copy.
        let backed = recovery_dir(&beads)
            .join(&manifest.run_id)
            .join("beads.db");
        write(&backed, b"tampered-different-length");

        let err = verify_run(&beads, &manifest.run_id).unwrap_err();
        assert!(err.to_string().contains("failed hash verification"));
    }

    #[test]
    fn restore_replaces_live_files() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        let db = beads.join("beads.db");
        write(&db, b"good-state");
        let manifest = create_backup(&beads, &[db.clone()], "pre-rebuild").unwrap();

        // Live db gets clobbered by a "failed rebuild".
        write(&db, b"corrupt-rebuild-output");
        assert_eq!(std::fs::read(&db).unwrap(), b"corrupt-rebuild-output");

        let restored = restore_run(&beads, &manifest.run_id).unwrap();
        assert_eq!(restored.run_id, manifest.run_id);
        assert_eq!(std::fs::read(&db).unwrap(), b"good-state");
    }

    #[test]
    fn restore_latest_picks_newest_run() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        let db = beads.join("beads.db");

        write(&db, b"v1");
        let first = create_backup(&beads, &[db.clone()], "pre-rebuild").unwrap();
        // Force a strictly-later run id so ordering is deterministic without a clock race.
        let root = recovery_dir(&beads);
        let second_id = format!("{}-zzz", first.run_id);
        let second_dir = root.join(&second_id);
        std::fs::create_dir_all(&second_dir).unwrap();
        write(&second_dir.join("beads.db"), b"v2");
        let second_manifest = BackupManifest {
            run_id: second_id.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            reason: "pre-rebuild".to_string(),
            files: vec![BackedUpFile {
                name: "beads.db".to_string(),
                sha256: hash_file(&second_dir.join("beads.db")).unwrap(),
                bytes: 2,
            }],
        };
        std::fs::write(
            second_dir.join(MANIFEST_NAME),
            serde_json::to_string_pretty(&second_manifest).unwrap(),
        )
        .unwrap();

        let runs = list_runs(&beads).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].run_id, second_id, "newest run should sort first");

        restore_run(&beads, "latest").unwrap();
        assert_eq!(std::fs::read(&db).unwrap(), b"v2");
    }

    #[test]
    fn marker_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let beads = tmp.path().join(".beads");
        std::fs::create_dir_all(&beads).unwrap();

        assert!(!repair_failed_marker_exists(&beads));
        write_repair_failed_marker(&beads, "restored run-x").unwrap();
        assert!(repair_failed_marker_exists(&beads));
        clear_repair_failed_marker(&beads).unwrap();
        assert!(!repair_failed_marker_exists(&beads));
    }
}
