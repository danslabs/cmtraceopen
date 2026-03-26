use tauri::AppHandle;

use crate::collector::types::CollectionResult;

/// Collect system diagnostics into an evidence bundle.
///
/// This is a long-running command that emits `collection-progress` events
/// to the frontend as artifacts are collected. All five collection categories
/// (logs, registry, event logs, exports, commands) run concurrently.
///
/// Windows-only. Returns an error on other platforms.
#[tauri::command]
pub async fn collect_diagnostics(
    request_id: String,
    output_root: Option<String>,
    app: AppHandle,
) -> Result<CollectionResult, String> {
    collect_diagnostics_impl(request_id, output_root, app).await
}

#[cfg(target_os = "windows")]
async fn collect_diagnostics_impl(
    request_id: String,
    output_root: Option<String>,
    app: AppHandle,
) -> Result<CollectionResult, String> {
    tokio::task::spawn_blocking(move || {
        crate::collector::engine::run_collection(request_id, output_root, app)
    })
    .await
    .map_err(|e| format!("collection task panicked: {e}"))?
}

#[cfg(not(target_os = "windows"))]
async fn collect_diagnostics_impl(
    _request_id: String,
    _output_root: Option<String>,
    _app: AppHandle,
) -> Result<CollectionResult, String> {
    Err("Diagnostics collection is only supported on Windows.".to_string())
}
