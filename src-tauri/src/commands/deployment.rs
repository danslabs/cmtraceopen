//! Software Deployment workspace backend.
//!
//! Scans a folder recursively for deployment logs (MSI, PSADT, Burn, PatchMyPC),
//! classifies each file's format and outcome, extracts exit codes and error context,
//! and returns structured results for the frontend workspace.

use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;
use std::path::Path;

use crate::error_db::lookup::lookup_error_code;
use crate::models::log_entry::{LogEntry, ParserKind, Severity};
use crate::parser;

// ── Types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub enum DeploymentFormat {
    #[serde(rename = "psadt-cmtrace")]
    PsadtCmtrace,
    #[serde(rename = "psadt-legacy")]
    PsadtLegacy,
    #[serde(rename = "msi-verbose")]
    MsiVerbose,
    #[serde(rename = "psadt-wrapper")]
    PsadtWrapper,
    #[serde(rename = "burn")]
    Burn,
    #[serde(rename = "patchmypc")]
    PatchMyPc,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum DeploymentOutcome {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "deferred")]
    Deferred,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentErrorLine {
    pub line_number: u32,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentLogFile {
    pub path: String,
    pub file_name: String,
    pub format: DeploymentFormat,
    pub outcome: DeploymentOutcome,
    pub exit_code: Option<i32>,
    pub error_summary: Option<String>,
    pub error_lines: Vec<DeploymentErrorLine>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentAnalysisResult {
    pub folder_path: String,
    pub files: Vec<DeploymentLogFile>,
    pub total_files: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub deferred: usize,
    pub unknown: usize,
}

// ── Regex patterns ───────────────────────────────────────────────────────

static MSI_MAIN_ENGINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"MainEngineThread is returning (\d+)").unwrap()
});

static MSI_RETURN_VALUE_3_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"Return value 3\b").unwrap()
});

static PSADT_EXIT_CODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)exit\s*code\s*[\[:\s]*(\d+)").unwrap()
});

static BURN_RETURN_CODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:error|return)\s*code[:\s]+(\d+)").unwrap()
});

// ── PSADT keywords ──────────────────────────────────────────────────────

const PSADT_KEYWORDS: &[&str] = &[
    "Open-ADTSession",
    "Close-ADTSession",
    "PSAppDeployToolkit",
    "Start-ADTMsiProcess",
    "ADTSession",
];

// ── Format classification ───────────────────────────────────────────────

fn classify_format(
    parser_kind: ParserKind,
    entries: &[LogEntry],
    file_path: &str,
) -> DeploymentFormat {
    match parser_kind {
        ParserKind::Msi => DeploymentFormat::MsiVerbose,
        ParserKind::PsadtLegacy => DeploymentFormat::PsadtLegacy,
        ParserKind::Burn => DeploymentFormat::Burn,
        ParserKind::Ccm => {
            let has_psadt = entries.iter().any(|e| {
                PSADT_KEYWORDS.iter().any(|kw| e.message.contains(kw))
                    || e.component
                        .as_deref()
                        .is_some_and(|c| PSADT_KEYWORDS.iter().any(|kw| c.contains(kw)))
            });
            if has_psadt {
                DeploymentFormat::PsadtCmtrace
            } else if file_path.to_ascii_lowercase().contains("patchmypc") {
                DeploymentFormat::PatchMyPc
            } else {
                DeploymentFormat::PsadtWrapper
            }
        }
        _ => DeploymentFormat::Unknown,
    }
}

// ── Exit code extraction ────────────────────────────────────────────────

fn extract_exit_code(format: &DeploymentFormat, entries: &[LogEntry]) -> Option<i32> {
    match format {
        DeploymentFormat::MsiVerbose => {
            for entry in entries.iter().rev() {
                if let Some(caps) = MSI_MAIN_ENGINE_RE.captures(&entry.message) {
                    if let Ok(code) = caps[1].parse::<i32>() {
                        return Some(code);
                    }
                }
            }
            None
        }
        DeploymentFormat::PsadtCmtrace | DeploymentFormat::PsadtWrapper | DeploymentFormat::PatchMyPc => {
            // Search for Close-ADTSession with exit code
            for entry in entries.iter().rev() {
                if entry.message.contains("Close-ADTSession") || entry.message.contains("ADTSession") {
                    if let Some(caps) = PSADT_EXIT_CODE_RE.captures(&entry.message) {
                        if let Ok(code) = caps[1].parse::<i32>() {
                            return Some(code);
                        }
                    }
                }
            }
            // Fallback: any exit code pattern
            for entry in entries.iter().rev() {
                if let Some(caps) = PSADT_EXIT_CODE_RE.captures(&entry.message) {
                    if let Ok(code) = caps[1].parse::<i32>() {
                        return Some(code);
                    }
                }
            }
            None
        }
        DeploymentFormat::PsadtLegacy => {
            // Component field is the source function name
            for entry in entries.iter().rev() {
                let is_close = entry
                    .component
                    .as_deref()
                    .is_some_and(|c| c.contains("Close-ADTSession"));
                if is_close {
                    if let Some(caps) = PSADT_EXIT_CODE_RE.captures(&entry.message) {
                        if let Ok(code) = caps[1].parse::<i32>() {
                            return Some(code);
                        }
                    }
                }
            }
            for entry in entries.iter().rev() {
                if let Some(caps) = PSADT_EXIT_CODE_RE.captures(&entry.message) {
                    if let Ok(code) = caps[1].parse::<i32>() {
                        return Some(code);
                    }
                }
            }
            None
        }
        DeploymentFormat::Burn => {
            for entry in entries.iter().rev() {
                if entry.severity == Severity::Error {
                    if let Some(caps) = BURN_RETURN_CODE_RE.captures(&entry.message) {
                        if let Ok(code) = caps[1].parse::<i32>() {
                            return Some(code);
                        }
                    }
                }
            }
            None
        }
        DeploymentFormat::Unknown => None,
    }
}

// ── Outcome classification ──────────────────────────────────────────────

fn classify_outcome(exit_code: Option<i32>) -> DeploymentOutcome {
    match exit_code {
        Some(0) | Some(3010) | Some(1641) => DeploymentOutcome::Success,
        Some(1602) | Some(1604) | Some(60012) | Some(70001) => DeploymentOutcome::Deferred,
        Some(_) => DeploymentOutcome::Failure,
        None => DeploymentOutcome::Unknown,
    }
}

// ── Error context extraction ────────────────────────────────────────────

fn extract_error_lines(format: &DeploymentFormat, entries: &[LogEntry]) -> Vec<DeploymentErrorLine> {
    let mut lines = Vec::new();

    match format {
        DeploymentFormat::MsiVerbose => {
            // Find "Return value 3" lines with context
            for (i, entry) in entries.iter().enumerate() {
                if MSI_RETURN_VALUE_3_RE.is_match(&entry.message) {
                    let start = i.saturating_sub(3);
                    for ctx in &entries[start..=i] {
                        lines.push(DeploymentErrorLine {
                            line_number: ctx.line_number,
                            message: ctx.message.clone(),
                            severity: "Error".to_string(),
                        });
                    }
                }
            }
            // Include MainEngineThread line
            for entry in entries.iter() {
                if MSI_MAIN_ENGINE_RE.is_match(&entry.message) {
                    lines.push(DeploymentErrorLine {
                        line_number: entry.line_number,
                        message: entry.message.clone(),
                        severity: "Error".to_string(),
                    });
                }
            }
        }
        _ => {
            for entry in entries.iter() {
                match entry.severity {
                    Severity::Error => {
                        lines.push(DeploymentErrorLine {
                            line_number: entry.line_number,
                            message: entry.message.clone(),
                            severity: "Error".to_string(),
                        });
                    }
                    Severity::Warning => {
                        lines.push(DeploymentErrorLine {
                            line_number: entry.line_number,
                            message: entry.message.clone(),
                            severity: "Warning".to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    lines.truncate(50);
    lines
}

// ── Error summary ───────────────────────────────────────────────────────

fn generate_error_summary(
    format: &DeploymentFormat,
    exit_code: Option<i32>,
    outcome: &DeploymentOutcome,
) -> Option<String> {
    match outcome {
        DeploymentOutcome::Success | DeploymentOutcome::Unknown => return None,
        _ => {}
    }

    let code = exit_code?;
    let lookup = lookup_error_code(&code.to_string());

    let prefix = match format {
        DeploymentFormat::MsiVerbose => "MSI",
        DeploymentFormat::PsadtCmtrace | DeploymentFormat::PsadtLegacy | DeploymentFormat::PsadtWrapper => "PSADT",
        DeploymentFormat::Burn => "Burn",
        DeploymentFormat::PatchMyPc => "PatchMyPC",
        DeploymentFormat::Unknown => "Deployment",
    };

    if lookup.found {
        Some(format!("{} exit code {}: {}", prefix, code, lookup.description))
    } else {
        Some(format!("{} exit code {}", prefix, code))
    }
}

// ── Single file analysis ────────────────────────────────────────────────

fn analyze_single_file(file_path: &str) -> DeploymentLogFile {
    let path_obj = Path::new(file_path);
    let file_name = path_obj
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    match parser::parse_file(file_path) {
        Ok((result, resolved)) => {
            let format = classify_format(resolved.parser, &result.entries, file_path);
            let exit_code = extract_exit_code(&format, &result.entries);
            let outcome = classify_outcome(exit_code);
            let error_summary = generate_error_summary(&format, exit_code, &outcome);
            let error_lines = match outcome {
                DeploymentOutcome::Failure | DeploymentOutcome::Deferred => {
                    extract_error_lines(&format, &result.entries)
                }
                _ => Vec::new(),
            };

            DeploymentLogFile {
                path: file_path.to_string(),
                file_name,
                format,
                outcome,
                exit_code,
                error_summary,
                error_lines,
            }
        }
        Err(_) => DeploymentLogFile {
            path: file_path.to_string(),
            file_name,
            format: DeploymentFormat::Unknown,
            outcome: DeploymentOutcome::Unknown,
            exit_code: None,
            error_summary: None,
            error_lines: Vec::new(),
        },
    }
}

// ── Recursive file enumeration ──────────────────────────────────────────

fn collect_log_files(dir: &Path, out: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_log_files(&path, out);
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_ascii_lowercase() == "log" {
                    out.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
}

// ── Tauri command ───────────────────────────────────────────────────────

#[tauri::command]
pub fn analyze_deployment_folder(
    folder_path: String,
) -> Result<DeploymentAnalysisResult, String> {
    let dir = Path::new(&folder_path);
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", folder_path));
    }

    let mut log_files = Vec::new();
    collect_log_files(dir, &mut log_files);

    if log_files.is_empty() {
        return Ok(DeploymentAnalysisResult {
            folder_path,
            files: Vec::new(),
            total_files: 0,
            succeeded: 0,
            failed: 0,
            deferred: 0,
            unknown: 0,
        });
    }

    // Parse all files in parallel
    let files: Vec<DeploymentLogFile> = log_files
        .par_iter()
        .map(|p| analyze_single_file(p))
        .collect();

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut deferred = 0usize;
    let mut unknown = 0usize;

    for file in &files {
        match file.outcome {
            DeploymentOutcome::Success => succeeded += 1,
            DeploymentOutcome::Failure => failed += 1,
            DeploymentOutcome::Deferred => deferred += 1,
            DeploymentOutcome::Unknown => unknown += 1,
        }
    }

    let total_files = files.len();

    Ok(DeploymentAnalysisResult {
        folder_path,
        files,
        total_files,
        succeeded,
        failed,
        deferred,
        unknown,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_entry::LogFormat;

    fn make_entry(msg: &str, sev: Severity) -> LogEntry {
        LogEntry {
            id: 0,
            line_number: 1,
            message: msg.to_string(),
            component: None,
            timestamp: None,
            timestamp_display: None,
            severity: sev,
            thread: None,
            thread_display: None,
            source_file: None,
            format: LogFormat::Plain,
            file_path: "test.log".to_string(),
            timezone_offset: None,
            error_code_spans: Vec::new(),
            ip_address: None,
            host_name: None,
            mac_address: None,
        }
    }

    #[test]
    fn test_outcome_success() {
        assert!(matches!(classify_outcome(Some(0)), DeploymentOutcome::Success));
        assert!(matches!(classify_outcome(Some(3010)), DeploymentOutcome::Success));
        assert!(matches!(classify_outcome(Some(1641)), DeploymentOutcome::Success));
    }

    #[test]
    fn test_outcome_deferred() {
        assert!(matches!(classify_outcome(Some(1602)), DeploymentOutcome::Deferred));
        assert!(matches!(classify_outcome(Some(60012)), DeploymentOutcome::Deferred));
    }

    #[test]
    fn test_outcome_failure() {
        assert!(matches!(classify_outcome(Some(1603)), DeploymentOutcome::Failure));
        assert!(matches!(classify_outcome(Some(1)), DeploymentOutcome::Failure));
    }

    #[test]
    fn test_outcome_unknown() {
        assert!(matches!(classify_outcome(None), DeploymentOutcome::Unknown));
    }

    #[test]
    fn test_msi_exit_code() {
        let entries = vec![make_entry("MainEngineThread is returning 1603", Severity::Error)];
        assert_eq!(extract_exit_code(&DeploymentFormat::MsiVerbose, &entries), Some(1603));
    }

    #[test]
    fn test_psadt_exit_code() {
        let entries = vec![make_entry("Close-ADTSession completed with exit code [0]", Severity::Info)];
        assert_eq!(extract_exit_code(&DeploymentFormat::PsadtCmtrace, &entries), Some(0));
    }

    #[test]
    fn test_format_msi_direct() {
        let entries = vec![make_entry("test", Severity::Info)];
        assert!(matches!(classify_format(ParserKind::Msi, &entries, "test.log"), DeploymentFormat::MsiVerbose));
    }

    #[test]
    fn test_format_burn_direct() {
        let entries = vec![make_entry("test", Severity::Info)];
        assert!(matches!(classify_format(ParserKind::Burn, &entries, "test.log"), DeploymentFormat::Burn));
    }

    #[test]
    fn test_format_ccm_with_psadt() {
        let entries = vec![make_entry("Open-ADTSession starting", Severity::Info)];
        assert!(matches!(classify_format(ParserKind::Ccm, &entries, "test.log"), DeploymentFormat::PsadtCmtrace));
    }

    #[test]
    fn test_format_ccm_patchmypc() {
        let entries = vec![make_entry("starting up", Severity::Info)];
        assert!(matches!(
            classify_format(ParserKind::Ccm, &entries, "C:\\PatchMyPC\\Logs\\test.log"),
            DeploymentFormat::PatchMyPc
        ));
    }

    #[test]
    fn test_error_summary_with_known_code() {
        let summary = generate_error_summary(&DeploymentFormat::MsiVerbose, Some(1603), &DeploymentOutcome::Failure);
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("1603"));
    }

    #[test]
    fn test_error_summary_success_none() {
        assert!(generate_error_summary(&DeploymentFormat::MsiVerbose, Some(0), &DeploymentOutcome::Success).is_none());
    }
}
