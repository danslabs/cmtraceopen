use std::fs;
use std::path::Path;

use chrono::Utc;
use serde_json::json;

use crate::collector::types::{ArtifactCounts, ArtifactResult, ArtifactStatus, CollectionProfile};

/// Write `manifest.json` into the bundle root, compatible with the existing
/// `inspect_evidence_bundle` logic in `file_ops.rs`.
pub fn write_manifest(
    bundle_root: &Path,
    profile: &CollectionProfile,
    results: &[ArtifactResult],
    counts: &ArtifactCounts,
    duration_ms: u64,
) -> Result<(), String> {
    let now = Utc::now();
    let hostname = hostname();

    let gaps: Vec<serde_json::Value> = results
        .iter()
        .filter(|r| !matches!(r.status, ArtifactStatus::Collected))
        .map(|r| {
            json!({
                "artifactId": r.id,
                "category": r.category,
                "status": format!("{:?}", r.status),
                "reason": r.error.as_deref().unwrap_or("unknown"),
            })
        })
        .collect();

    let manifest = json!({
        "manifestVersion": 1,
        "manifestPath": "manifest.json",
        "source": "cmtrace-open-collector",
        "profileName": profile.profile_name,
        "profileVersion": profile.profile_version,
        "collectedAt": now.to_rfc3339(),
        "hostname": hostname,
        "durationMs": duration_ms,
        "artifacts": {
            "collected": counts.collected,
            "missing": counts.missing,
            "failed": counts.failed,
            "total": counts.total,
        },
        "gaps": gaps,
    });

    let manifest_path = bundle_root.join("manifest.json");
    let json_str = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize manifest: {e}"))?;
    fs::write(&manifest_path, json_str)
        .map_err(|e| format!("failed to write manifest at '{}': {e}", manifest_path.display()))?;

    Ok(())
}

/// Write `notes.md` into the bundle root with collection summary.
pub fn write_notes(
    bundle_root: &Path,
    profile: &CollectionProfile,
    counts: &ArtifactCounts,
    duration_ms: u64,
) -> Result<(), String> {
    let now = Utc::now();
    let hostname = hostname();

    let notes = format!(
        "# Evidence Collection Notes\n\
         \n\
         - **Collected by:** CMTrace Open (Rust collector)\n\
         - **Profile:** {} v{}\n\
         - **Device:** {}\n\
         - **Timestamp:** {}\n\
         - **Duration:** {:.1}s\n\
         \n\
         ## Summary\n\
         \n\
         | Metric | Count |\n\
         |--------|-------|\n\
         | Collected | {} |\n\
         | Missing | {} |\n\
         | Failed | {} |\n\
         | **Total** | **{}** |\n\
         \n\
         ## Structure\n\
         \n\
         ```\n\
         evidence/\n\
         ├── logs/           Log files (IME, Panther, CBS, MSI, etc.)\n\
         ├── registry/       Registry exports (.reg)\n\
         ├── event-logs/     Event log copies (.evtx)\n\
         ├── exports/        Configuration files and diagnostic outputs\n\
         └── command-output/  Command stdout captures\n\
         ```\n",
        profile.profile_name,
        profile.profile_version,
        hostname,
        now.format("%Y-%m-%d %H:%M:%S UTC"),
        duration_ms as f64 / 1000.0,
        counts.collected,
        counts.missing,
        counts.failed,
        counts.total,
    );

    let notes_path = bundle_root.join("notes.md");
    fs::write(&notes_path, notes)
        .map_err(|e| format!("failed to write notes at '{}': {e}", notes_path.display()))?;

    Ok(())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
