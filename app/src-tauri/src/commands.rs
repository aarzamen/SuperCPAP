use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::analysis::lab::{lab_feature_catalog, LabFeature};
use crate::analysis::session::{analyze_source_paths as build_analysis, AnalysisResult};
use crate::analysis::source_profile::{
    profile_source_paths as build_source_profile, SourceQualityProfile,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub total_files: usize,
    pub accepted_files: usize,
    pub rejected_files: usize,
    pub total_accepted_bytes: u64,
    pub entries: Vec<SourceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEntry {
    pub file_name: String,
    pub extension: String,
    pub byte_count: u64,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn summarize_source_paths(paths: Vec<String>) -> Result<SourceSummary, String> {
    let mut entries = Vec::new();
    let mut seen_files = HashSet::new();

    for path_text in paths {
        collect_source_entries(Path::new(&path_text), &mut entries, &mut seen_files)?;
    }
    entries.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then(left.extension.cmp(&right.extension))
            .then(left.byte_count.cmp(&right.byte_count))
    });

    let accepted_files = entries.iter().filter(|entry| entry.accepted).count();
    let rejected_files = entries.len().saturating_sub(accepted_files);
    let total_accepted_bytes = entries
        .iter()
        .filter(|entry| entry.accepted)
        .map(|entry| entry.byte_count)
        .sum();

    Ok(SourceSummary {
        total_files: entries.len(),
        accepted_files,
        rejected_files,
        total_accepted_bytes,
        entries,
    })
}

#[tauri::command]
pub fn get_lab_features() -> Vec<LabFeature> {
    lab_feature_catalog()
}

#[tauri::command]
pub fn profile_source_paths(paths: Vec<String>) -> Result<SourceQualityProfile, String> {
    let local_paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    build_source_profile(&local_paths)
}

#[tauri::command]
pub fn analyze_source_paths(paths: Vec<String>) -> Result<AnalysisResult, String> {
    let local_paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    build_analysis(&local_paths)
}

fn collect_source_entries(
    path: &Path,
    entries: &mut Vec<SourceEntry>,
    seen_files: &mut HashSet<std::path::PathBuf>,
) -> Result<(), String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            entries.push(missing_entry(path));
            return Ok(());
        }
    };

    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Ok(());
    }

    if file_type.is_file() {
        if let Ok(canonical_path) = path.canonicalize() {
            if !seen_files.insert(canonical_path) {
                return Ok(());
            }
        }
        entries.push(summarize_one_path(path));
        return Ok(());
    }

    if file_type.is_dir() {
        let mut children = std::fs::read_dir(path)
            .map_err(|error| format!("failed to read selected folder: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to inspect selected folder: {error}"))?;
        children.sort_by_key(|entry| entry.file_name());

        for child in children {
            collect_source_entries(&child.path(), entries, seen_files)?;
        }
        return Ok(());
    }

    entries.push(SourceEntry {
        file_name: file_name(path),
        extension: extension(path),
        byte_count: 0,
        accepted: false,
        reason: Some("not a file".into()),
    });
    Ok(())
}

fn summarize_one_path(path: &Path) -> SourceEntry {
    let file_name = file_name(path);
    let extension = extension(path);

    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return missing_entry(path);
        }
    };

    if !metadata.is_file() {
        return SourceEntry {
            file_name,
            extension,
            byte_count: 0,
            accepted: false,
            reason: Some("not a file".into()),
        };
    }

    let accepted = matches!(extension.as_str(), "edf" | "crc" | "csv" | "tsv");
    SourceEntry {
        file_name,
        extension,
        byte_count: metadata.len(),
        accepted,
        reason: if accepted {
            None
        } else {
            Some("unsupported extension".into())
        },
    }
}

fn missing_entry(path: &Path) -> SourceEntry {
    SourceEntry {
        file_name: file_name(path),
        extension: extension(path),
        byte_count: 0,
        accepted: false,
        reason: Some("file not found".into()),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::summarize_source_paths;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn summarizes_local_supported_files_without_returning_full_paths() {
        let dir = make_temp_dir();
        let edf_path = dir.join("20250914_211945_BRP.edf");
        let crc_path = dir.join("20250914_211945_BRP.crc");
        let txt_path = dir.join("notes.txt");
        fs::write(&edf_path, [1_u8, 2, 3, 4]).expect("write edf fixture");
        fs::write(&crc_path, [5_u8, 6]).expect("write crc fixture");
        fs::write(&txt_path, [7_u8]).expect("write txt fixture");

        let summary = summarize_source_paths(vec![
            edf_path.display().to_string(),
            crc_path.display().to_string(),
            txt_path.display().to_string(),
        ])
        .expect("summary succeeds");

        assert_eq!(summary.total_files, 3);
        assert_eq!(summary.accepted_files, 2);
        assert_eq!(summary.rejected_files, 1);
        assert_eq!(summary.total_accepted_bytes, 6);
        let edf_entry = summary
            .entries
            .iter()
            .find(|entry| entry.file_name == "20250914_211945_BRP.edf")
            .expect("expected edf entry");
        assert!(!edf_entry.file_name.contains(dir.to_string_lossy().as_ref()));
        assert_eq!(summary.entries[2].accepted, false);
        assert_eq!(
            summary.entries[2].reason.as_deref(),
            Some("unsupported extension")
        );

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn recursively_summarizes_multiple_folders_and_files_without_following_symlinks() {
        let left = make_temp_dir();
        let right = make_temp_dir();
        let nested = left.join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");

        let loose_edf = right.join("20250914_211945_SAD.edf");
        let nested_edf = nested.join("20250914_211945_PLD.edf");
        let nested_crc = nested.join("20250914_211945_PLD.crc");
        let unsupported = left.join("notes.rtf");
        fs::write(&loose_edf, [1_u8, 2, 3]).expect("write loose edf");
        fs::write(&nested_edf, [4_u8, 5, 6, 7]).expect("write nested edf");
        fs::write(&nested_crc, [8_u8]).expect("write nested crc");
        fs::write(&unsupported, [9_u8]).expect("write unsupported file");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&right, left.join("right-symlink"))
            .expect("create directory symlink");

        let summary = summarize_source_paths(vec![
            left.display().to_string(),
            right.display().to_string(),
        ])
        .expect("summary succeeds");

        assert_eq!(summary.accepted_files, 3);
        assert_eq!(summary.rejected_files, 1);
        assert_eq!(summary.total_accepted_bytes, 8);
        assert!(summary
            .entries
            .iter()
            .any(|entry| entry.file_name == "20250914_211945_PLD.edf"));
        assert!(summary
            .entries
            .iter()
            .any(|entry| entry.file_name == "20250914_211945_PLD.crc"));
        assert!(summary
            .entries
            .iter()
            .any(|entry| entry.file_name == "20250914_211945_SAD.edf"));
        assert!(summary
            .entries
            .iter()
            .any(|entry| entry.file_name == "notes.rtf" && !entry.accepted));
        assert!(!summary
            .entries
            .iter()
            .any(|entry| entry.file_name == "right-symlink"));

        fs::remove_dir_all(left).ok();
        fs::remove_dir_all(right).ok();
    }

    fn make_temp_dir() -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        let counter = NEXT_TEMP_DIR.fetch_add(1, Ordering::SeqCst);
        dir.push(format!(
            "aerie-source-summary-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
