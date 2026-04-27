use crate::analysis::edf::{parse_edf, DecodedChannel, EdfRole, ParsedEdf};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceQualityProfile {
    pub total_files: usize,
    pub supported_files: usize,
    pub rejected_files: usize,
    pub edf_files: usize,
    pub crc_files: usize,
    pub csv_files: usize,
    pub tsv_files: usize,
    pub valid_edf_files: usize,
    pub limited_edf_files: usize,
    pub parse_error_edf_files: usize,
    pub role_counts: RoleCounts,
    pub valid_role_counts: RoleCounts,
    pub complete_sessions: usize,
    pub oximetry: OximetrySummary,
    pub best_session: Option<BestSession>,
    pub recommendation: FixtureRecommendation,
    pub strengths: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleCounts {
    pub brp: usize,
    pub pld: usize,
    pub sad: usize,
    pub eve: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OximetrySummary {
    pub valid_sad_files: usize,
    pub sentinel_only_sad_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestSession {
    pub start_date: String,
    pub start_time: String,
    pub duration_seconds: u64,
    pub files: BestSessionFiles,
    pub signals: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BestSessionFiles {
    pub brp: String,
    pub pld: String,
    pub sad: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureRecommendation {
    pub status: FixtureStatus,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureStatus {
    Strong,
    Partial,
    Weak,
}

#[derive(Debug, Clone)]
struct CandidateEdf {
    file_name: String,
    role: EdfRole,
    start_date: String,
    start_time: String,
    start_seconds: Option<i64>,
    duration_seconds: u64,
    has_flow: bool,
    has_pressure: bool,
    has_leak: bool,
    has_valid_oximetry: bool,
    has_sentinel_oximetry: bool,
}

#[derive(Debug, Clone)]
struct SessionCandidate {
    brp: CandidateEdf,
    pld: CandidateEdf,
    sad: CandidateEdf,
    duration_seconds: u64,
}

pub fn profile_source_paths(paths: &[PathBuf]) -> Result<SourceQualityProfile, String> {
    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    for path in paths {
        collect_files(path, &mut files, &mut seen_files)?;
    }
    files.sort();

    let mut profile = SourceQualityProfile {
        total_files: files.len(),
        supported_files: 0,
        rejected_files: 0,
        edf_files: 0,
        crc_files: 0,
        csv_files: 0,
        tsv_files: 0,
        valid_edf_files: 0,
        limited_edf_files: 0,
        parse_error_edf_files: 0,
        role_counts: RoleCounts::default(),
        valid_role_counts: RoleCounts::default(),
        complete_sessions: 0,
        oximetry: OximetrySummary::default(),
        best_session: None,
        recommendation: FixtureRecommendation {
            status: FixtureStatus::Weak,
            title: "No usable PAP fixture yet".into(),
            summary: "Select a folder containing EDF/CRC exports to profile it locally.".into(),
        },
        strengths: Vec::new(),
        limitations: Vec::new(),
    };

    let mut candidates = Vec::new();

    for path in files {
        match extension(&path).as_str() {
            "edf" => {
                profile.supported_files += 1;
                profile.edf_files += 1;
                profile_one_edf(&path, &mut profile, &mut candidates);
            }
            "crc" => {
                profile.supported_files += 1;
                profile.crc_files += 1;
            }
            "csv" => {
                profile.supported_files += 1;
                profile.csv_files += 1;
            }
            "tsv" => {
                profile.supported_files += 1;
                profile.tsv_files += 1;
            }
            _ => {
                profile.rejected_files += 1;
            }
        }
    }

    let sessions = complete_sessions(&candidates);
    profile.complete_sessions = sessions.len();
    profile.best_session = sessions
        .iter()
        .max_by_key(|session| session.duration_seconds)
        .map(best_session_from_candidate);
    profile.strengths = strengths(&profile);
    profile.limitations = limitations(&profile);
    profile.recommendation = recommendation(&profile);

    Ok(profile)
}

fn profile_one_edf(
    path: &Path,
    profile: &mut SourceQualityProfile,
    candidates: &mut Vec<CandidateEdf>,
) {
    let file_name = file_name(path);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => {
            profile.parse_error_edf_files += 1;
            return;
        }
    };

    let parsed = match parse_edf(&file_name, &bytes) {
        Ok(parsed) => parsed,
        Err(_) => {
            profile.parse_error_edf_files += 1;
            return;
        }
    };

    increment_role(&mut profile.role_counts, &parsed.role);
    if parsed.valid {
        profile.valid_edf_files += 1;
        increment_role(&mut profile.valid_role_counts, &parsed.role);
        let candidate = candidate_from_parsed(&parsed);
        if candidate.role == EdfRole::Sad {
            if candidate.has_valid_oximetry {
                profile.oximetry.valid_sad_files += 1;
            } else if candidate.has_sentinel_oximetry {
                profile.oximetry.sentinel_only_sad_files += 1;
            }
        }
        candidates.push(candidate);
    } else if parsed.limited {
        profile.limited_edf_files += 1;
    }
}

fn candidate_from_parsed(parsed: &ParsedEdf) -> CandidateEdf {
    CandidateEdf {
        file_name: parsed.file_name.clone(),
        role: parsed.role.clone(),
        start_date: parsed.header.start_date.clone(),
        start_time: parsed.header.start_time.clone(),
        start_seconds: timestamp_seconds(&parsed.header.start_date, &parsed.header.start_time),
        duration_seconds: duration_seconds(parsed),
        has_flow: parsed.channels.iter().any(is_usable_flow_channel),
        has_pressure: parsed.channels.iter().any(is_usable_pressure_channel),
        has_leak: parsed.channels.iter().any(is_usable_leak_channel),
        has_valid_oximetry: parsed.channels.iter().any(is_valid_oximetry_channel),
        has_sentinel_oximetry: parsed.channels.iter().any(is_sentinel_oximetry_channel),
    }
}

fn complete_sessions(candidates: &[CandidateEdf]) -> Vec<SessionCandidate> {
    let mut sessions = Vec::new();
    for brp in candidates
        .iter()
        .filter(|candidate| candidate.role == EdfRole::Brp)
    {
        let Some(pld) = nearest_role(candidates, EdfRole::Pld, brp.start_seconds) else {
            continue;
        };
        let Some(sad) = nearest_role(candidates, EdfRole::Sad, brp.start_seconds) else {
            continue;
        };

        let duration_seconds = brp
            .duration_seconds
            .min(pld.duration_seconds)
            .min(sad.duration_seconds);
        sessions.push(SessionCandidate {
            brp: brp.clone(),
            pld: pld.clone(),
            sad: sad.clone(),
            duration_seconds,
        });
    }
    sessions
}

fn nearest_role(
    candidates: &[CandidateEdf],
    role: EdfRole,
    target_seconds: Option<i64>,
) -> Option<&CandidateEdf> {
    let target_seconds = target_seconds?;
    candidates
        .iter()
        .filter(|candidate| candidate.role == role)
        .filter_map(|candidate| {
            let offset = (candidate.start_seconds? - target_seconds).abs();
            (offset <= 2).then_some((offset, candidate))
        })
        .min_by_key(|(offset, _candidate)| *offset)
        .map(|(_offset, candidate)| candidate)
}

fn best_session_from_candidate(session: &SessionCandidate) -> BestSession {
    let mut signals = Vec::new();
    if session.brp.has_flow {
        signals.push("flow waveform".into());
    }
    if session.pld.has_pressure {
        signals.push("pressure trend".into());
    }
    if session.pld.has_leak {
        signals.push("leak trend".into());
    }
    if session.sad.has_valid_oximetry {
        signals.push("valid oximetry".into());
    }

    let mut limitations = Vec::new();
    if !session.sad.has_valid_oximetry {
        limitations.push("Oximetry unavailable for this session".into());
    }

    BestSession {
        start_date: session.brp.start_date.clone(),
        start_time: session.brp.start_time.clone(),
        duration_seconds: session.duration_seconds,
        files: BestSessionFiles {
            brp: session.brp.file_name.clone(),
            pld: session.pld.file_name.clone(),
            sad: session.sad.file_name.clone(),
        },
        signals,
        limitations,
    }
}

fn strengths(profile: &SourceQualityProfile) -> Vec<String> {
    let mut strengths = Vec::new();
    if profile.complete_sessions > 0 {
        strengths.push(format!(
            "{} complete BRP/PLD/SAD session(s) found within a two-second start window.",
            profile.complete_sessions
        ));
    }
    if profile.valid_role_counts.brp > 0 {
        strengths.push(format!(
            "{} valid BRP file(s) are available for flow waveform work.",
            profile.valid_role_counts.brp
        ));
    }
    if profile.valid_role_counts.pld > 0 {
        strengths.push(format!(
            "{} valid PLD file(s) are available for pressure and leak summaries.",
            profile.valid_role_counts.pld
        ));
    }
    if profile.crc_files > 0 {
        strengths.push(format!(
            "{} CRC sidecar file(s) were found for future integrity checks.",
            profile.crc_files
        ));
    }
    strengths
}

fn limitations(profile: &SourceQualityProfile) -> Vec<String> {
    let mut limitations = Vec::new();
    if profile.complete_sessions == 0 {
        limitations.push("No complete BRP/PLD/SAD session group was found yet.".into());
    }
    if profile.oximetry.valid_sad_files == 0 && profile.valid_role_counts.sad > 0 {
        limitations.push(
            "Oximetry unavailable: SAD SpO2/Pulse channels decode as sentinel values only.".into(),
        );
    }
    if profile.limited_edf_files > 0 {
        limitations.push(format!(
            "{} EDF file(s) are incomplete or header-only and will not produce metrics.",
            profile.limited_edf_files
        ));
    }
    if profile.parse_error_edf_files > 0 {
        limitations.push(format!(
            "{} EDF file(s) could not be parsed and need format review.",
            profile.parse_error_edf_files
        ));
    }
    if profile.rejected_files > 0 {
        limitations.push(format!(
            "{} unsupported file(s) were ignored by the CPAP parser.",
            profile.rejected_files
        ));
    }
    limitations
}

fn recommendation(profile: &SourceQualityProfile) -> FixtureRecommendation {
    if profile.complete_sessions >= 3
        && profile.valid_role_counts.brp > 0
        && profile.valid_role_counts.pld > 0
        && profile.valid_role_counts.sad > 0
    {
        return FixtureRecommendation {
            status: FixtureStatus::Strong,
            title: "Strong local PAP fixture".into(),
            summary:
                "Use this as the primary local test set for flow, pressure, leak, grouping, and invalid-oximetry gating."
                    .into(),
        };
    }

    if profile.complete_sessions > 0 {
        return FixtureRecommendation {
            status: FixtureStatus::Partial,
            title: "Usable partial fixture".into(),
            summary:
                "This folder can exercise the parser, but it has too few complete sessions for broader analysis work."
                    .into(),
        };
    }

    FixtureRecommendation {
        status: FixtureStatus::Weak,
        title: "Weak fixture".into(),
        summary: "Use only for scanner and rejection behavior until complete sessions are added."
            .into(),
    }
}

fn collect_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
    seen_files: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
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
        files.push(path.to_path_buf());
        return Ok(());
    }
    if file_type.is_dir() {
        let mut children = std::fs::read_dir(path)
            .map_err(|error| format!("failed to read selected folder: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to inspect selected folder: {error}"))?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            collect_files(&child.path(), files, seen_files)?;
        }
    }
    Ok(())
}

fn increment_role(counts: &mut RoleCounts, role: &EdfRole) {
    match role {
        EdfRole::Brp => counts.brp += 1,
        EdfRole::Pld => counts.pld += 1,
        EdfRole::Sad => counts.sad += 1,
        EdfRole::Eve => counts.eve += 1,
        EdfRole::Unknown => counts.unknown += 1,
    }
}

fn duration_seconds(parsed: &ParsedEdf) -> u64 {
    if parsed.header.record_count <= 0 || parsed.header.record_duration_seconds <= 0.0 {
        return 0;
    }
    (parsed.header.record_count as f64 * parsed.header.record_duration_seconds).round() as u64
}

fn is_usable_flow_channel(channel: &DecodedChannel) -> bool {
    channel_name_contains(channel, "flow")
        && channel.invalid_reason.is_none()
        && !channel.values.is_empty()
}

fn is_usable_pressure_channel(channel: &DecodedChannel) -> bool {
    channel_name_contains(channel, "press")
        && channel.invalid_reason.is_none()
        && !channel.values.is_empty()
}

fn is_usable_leak_channel(channel: &DecodedChannel) -> bool {
    channel_name_contains(channel, "leak")
        && channel.invalid_reason.is_none()
        && !channel.values.is_empty()
}

fn is_valid_oximetry_channel(channel: &DecodedChannel) -> bool {
    is_oximetry_channel(channel)
        && channel.invalid_reason.is_none()
        && channel.values.iter().any(|value| *value > 0.0)
}

fn is_sentinel_oximetry_channel(channel: &DecodedChannel) -> bool {
    is_oximetry_channel(channel) && channel.invalid_reason.is_some()
}

fn is_oximetry_channel(channel: &DecodedChannel) -> bool {
    channel_name_contains(channel, "spo2") || channel_name_contains(channel, "pulse")
}

fn channel_name_contains(channel: &DecodedChannel, needle: &str) -> bool {
    channel.label.to_ascii_lowercase().contains(needle)
}

fn timestamp_seconds(date: &str, time: &str) -> Option<i64> {
    let date_parts = split_three(date)?;
    let time_parts = split_three(time)?;
    let year = if date_parts[2] >= 85 {
        1900 + date_parts[2]
    } else {
        2000 + date_parts[2]
    };
    let day = date_parts[0];
    let month = date_parts[1];
    let hour = time_parts[0];
    let minute = time_parts[1];
    let second = time_parts[2];

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    Some(
        days_from_civil(year as i64, month as i64, day as i64) * 86_400
            + hour as i64 * 3_600
            + minute as i64 * 60
            + second as i64,
    )
}

fn split_three(value: &str) -> Option<[u32; 3]> {
    let parts = value
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    (parts.len() == 3).then_some([parts[0], parts[1], parts[2]])
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
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
    use super::profile_source_paths;
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn profiles_complete_local_pap_fixture_without_exposing_paths() {
        let dir = make_temp_dir();
        write_synthetic_edf(
            &dir.join("20250816_233919_BRP.edf"),
            60,
            120.0,
            vec![SignalSpec::new(
                "Flow.40ms",
                "L/sec",
                -2.0,
                2.0,
                -32768,
                32767,
                3000,
            )],
            vec![vec![0; 3000]],
        );
        write_synthetic_edf(
            &dir.join("20250816_233920_PLD.edf"),
            60,
            120.0,
            vec![
                SignalSpec::new("Press.2s", "cmH2O", 0.0, 30.0, 0, 3000, 60),
                SignalSpec::new("Leak.2s", "L/sec", 0.0, 10.0, 0, 1000, 60),
            ],
            vec![vec![1000; 60], vec![0; 60]],
        );
        write_synthetic_edf(
            &dir.join("20250816_233920_SAD.edf"),
            60,
            120.0,
            vec![
                SignalSpec::new("SpO2.1s", "%", -1.0, 100.0, -1, 100, 120),
                SignalSpec::new("Pulse.1s", "bpm", -1.0, 250.0, -1, 250, 120),
            ],
            vec![vec![-1; 120], vec![-1; 120]],
        );
        fs::write(dir.join("20250816_233920_PLD.crc"), [1_u8, 2, 3]).expect("write crc");
        fs::write(dir.join(".DS_Store"), [4_u8]).expect("write unsupported");

        let profile = profile_source_paths(&[dir.clone()]).expect("profile succeeds");

        assert_eq!(profile.total_files, 5);
        assert_eq!(profile.supported_files, 4);
        assert_eq!(profile.rejected_files, 1);
        assert_eq!(profile.edf_files, 3);
        assert_eq!(profile.crc_files, 1);
        assert_eq!(profile.valid_edf_files, 3);
        assert_eq!(profile.role_counts.brp, 1);
        assert_eq!(profile.role_counts.pld, 1);
        assert_eq!(profile.role_counts.sad, 1);
        assert_eq!(profile.complete_sessions, 1);
        assert_eq!(profile.oximetry.valid_sad_files, 0);
        assert_eq!(profile.oximetry.sentinel_only_sad_files, 1);

        let best = profile.best_session.expect("best session selected");
        assert_eq!(best.duration_seconds, 7200);
        assert_eq!(best.files.brp, "20250816_233919_BRP.edf");
        assert_eq!(best.files.pld, "20250816_233920_PLD.edf");
        assert_eq!(best.files.sad, "20250816_233920_SAD.edf");
        assert!(!best.files.brp.contains(dir.to_string_lossy().as_ref()));
        assert!(profile
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Oximetry unavailable")));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    #[ignore = "reads the user's Desktop CPAP fixture folder when present"]
    fn profiles_desktop_untitled_folder_2_as_primary_local_fixture() {
        let fixture = Path::new("/Users/ama/Desktop/Untitled Folder 2");
        if !fixture.exists() {
            eprintln!("Desktop fixture folder is not present; skipping");
            return;
        }

        let profile = profile_source_paths(&[fixture.to_path_buf()]).expect("profile succeeds");

        assert_eq!(profile.edf_files, 159);
        assert_eq!(profile.crc_files, 159);
        assert!(profile.valid_edf_files >= 120);
        assert!(profile.complete_sessions >= 30);
        assert_eq!(profile.oximetry.valid_sad_files, 0);
        assert!(profile.oximetry.sentinel_only_sad_files >= 30);

        let best = profile.best_session.expect("best session selected");
        assert!(best.duration_seconds >= 7_000);
        assert_eq!(best.files.brp, "20250816_233919_BRP.edf");
        assert_eq!(best.files.pld, "20250816_233920_PLD.edf");
        assert_eq!(best.files.sad, "20250816_233920_SAD.edf");
    }

    fn make_temp_dir() -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        let counter = NEXT_TEMP_DIR.fetch_add(1, Ordering::SeqCst);
        dir.push(format!(
            "aerie-source-profile-{}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write_synthetic_edf(
        path: &Path,
        records: i64,
        record_duration_seconds: f64,
        signals: Vec<SignalSpec>,
        samples_per_record: Vec<Vec<i16>>,
    ) {
        let bytes = synthetic_edf(
            records,
            record_duration_seconds,
            signals,
            samples_per_record,
        );
        fs::write(path, bytes).unwrap_or_else(|error| {
            panic!("failed to write {}: {error}", path.display());
        });
    }

    struct SignalSpec {
        label: &'static str,
        unit: &'static str,
        physical_min: f64,
        physical_max: f64,
        digital_min: i16,
        digital_max: i16,
        samples_per_record: usize,
    }

    impl SignalSpec {
        fn new(
            label: &'static str,
            unit: &'static str,
            physical_min: f64,
            physical_max: f64,
            digital_min: i16,
            digital_max: i16,
            samples_per_record: usize,
        ) -> Self {
            Self {
                label,
                unit,
                physical_min,
                physical_max,
                digital_min,
                digital_max,
                samples_per_record,
            }
        }
    }

    fn synthetic_edf(
        records: i64,
        record_duration_seconds: f64,
        signals: Vec<SignalSpec>,
        samples_per_record: Vec<Vec<i16>>,
    ) -> Vec<u8> {
        let signal_count = signals.len();
        let header_bytes = 256 + signal_count * 256;
        let mut header = vec![b' '; header_bytes];

        write_field(&mut header, 0, 8, "0");
        write_field(&mut header, 8, 80, "LOCAL_PATIENT");
        write_field(&mut header, 88, 80, "LOCAL_RECORDING");
        write_field(&mut header, 168, 8, "16.08.25");
        write_field(&mut header, 176, 8, "23.39.20");
        write_field(&mut header, 184, 8, &header_bytes.to_string());
        write_field(&mut header, 236, 8, &records.to_string());
        write_field(
            &mut header,
            244,
            8,
            &format!("{record_duration_seconds:.1}"),
        );
        write_field(&mut header, 252, 4, &signal_count.to_string());

        let mut offset = 256;
        for signal in &signals {
            write_field(&mut header, offset, 16, signal.label);
            offset += 16;
        }
        for _signal in &signals {
            write_field(&mut header, offset, 80, "");
            offset += 80;
        }
        for signal in &signals {
            write_field(&mut header, offset, 8, signal.unit);
            offset += 8;
        }
        for signal in &signals {
            write_field(&mut header, offset, 8, &signal.physical_min.to_string());
            offset += 8;
        }
        for signal in &signals {
            write_field(&mut header, offset, 8, &signal.physical_max.to_string());
            offset += 8;
        }
        for signal in &signals {
            write_field(&mut header, offset, 8, &signal.digital_min.to_string());
            offset += 8;
        }
        for signal in &signals {
            write_field(&mut header, offset, 8, &signal.digital_max.to_string());
            offset += 8;
        }
        for _signal in &signals {
            write_field(&mut header, offset, 80, "");
            offset += 80;
        }
        for signal in &signals {
            write_field(
                &mut header,
                offset,
                8,
                &signal.samples_per_record.to_string(),
            );
            offset += 8;
        }
        for _signal in &signals {
            write_field(&mut header, offset, 32, "");
            offset += 32;
        }

        let mut bytes = header;
        if records > 0 {
            for _record in 0..records {
                for samples in &samples_per_record {
                    for sample in samples {
                        bytes.extend(sample.to_le_bytes());
                    }
                }
            }
        }
        bytes
    }

    fn write_field(bytes: &mut [u8], start: usize, width: usize, value: &str) {
        for (index, byte) in value.as_bytes().iter().take(width).enumerate() {
            bytes[start + index] = *byte;
        }
    }
}
