use crate::analysis::edf::{parse_edf, EdfRole, ParsedEdf};
use crate::analysis::findings::{finding_for_session, Finding};
use crate::analysis::lab_breath::breath_morphology_probe;
use crate::analysis::lab_common::LabProbeResult;
use crate::analysis::lab_oximetry_instability::{
    instability_windows_probe, oximetry_coupling_probe,
};
use crate::analysis::lab_pressure::{counterfactual_sandbox_probe, leak_pressure_probe};
use crate::analysis::lab_synchrony::trigger_cycle_synchrony_probe;
use crate::analysis::metrics::{metrics_from_triplet, SessionMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    Ready,
    Limited,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub status: AnalysisStatus,
    pub sessions: Vec<SessionAnalysis>,
    pub findings: Vec<Finding>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAnalysis {
    pub start_date: String,
    pub start_time: String,
    pub duration_seconds: u64,
    pub files: SessionFiles,
    pub metrics: SessionMetrics,
    pub findings: Vec<Finding>,
    pub lab_probes: Vec<LabProbeResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFiles {
    pub brp: String,
    pub pld: String,
    pub sad: String,
}

#[derive(Debug, Clone)]
struct SessionTriplet {
    brp: ParsedEdf,
    pld: ParsedEdf,
    sad: ParsedEdf,
}

pub fn analyze_source_paths(paths: &[PathBuf]) -> Result<AnalysisResult, String> {
    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    for path in paths {
        collect_files(path, &mut files, &mut seen_files)?;
    }
    files.sort();

    let mut parsed = Vec::new();
    let mut limitations = Vec::new();
    for path in files {
        if extension(&path) != "edf" {
            continue;
        }

        let file_name = file_name(&path);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                limitations.push(format!("{file_name} could not be read: {error}"));
                continue;
            }
        };
        match parse_edf(&file_name, &bytes) {
            Ok(edf) => parsed.push(edf),
            Err(error) => limitations.push(format!("{file_name} could not be parsed: {error}")),
        }
    }

    Ok(analysis_from_parsed_edfs_with_limitations(
        parsed,
        limitations,
    ))
}

pub fn analysis_from_parsed_edfs(parsed: Vec<ParsedEdf>) -> AnalysisResult {
    analysis_from_parsed_edfs_with_limitations(parsed, Vec::new())
}

fn analysis_from_parsed_edfs_with_limitations(
    parsed: Vec<ParsedEdf>,
    mut limitations: Vec<String>,
) -> AnalysisResult {
    let invalid_count = parsed.iter().filter(|edf| !edf.valid).count();
    if invalid_count > 0 {
        limitations.push(format!(
            "{invalid_count} EDF file(s) are incomplete or header-only and were excluded from metrics."
        ));
    }

    let triplets = complete_triplets(
        parsed
            .into_iter()
            .filter(|edf| edf.valid)
            .collect::<Vec<_>>(),
    );
    let mut sessions = triplets
        .into_iter()
        .map(session_from_triplet)
        .collect::<Vec<_>>();
    sessions.sort_by_key(|session| {
        timestamp_seconds(&session.start_date, &session.start_time).unwrap_or_default()
    });

    if sessions.is_empty() {
        limitations.push("No complete BRP/PLD/SAD session group was available for metrics.".into());
    }

    let findings = sessions
        .iter()
        .flat_map(|session| session.findings.clone())
        .collect::<Vec<_>>();
    let status = if !sessions.is_empty() {
        AnalysisStatus::Ready
    } else if limitations.is_empty() {
        AnalysisStatus::Empty
    } else {
        AnalysisStatus::Limited
    };

    AnalysisResult {
        status,
        sessions,
        findings,
        limitations,
    }
}

fn session_from_triplet(triplet: SessionTriplet) -> SessionAnalysis {
    let metrics = metrics_from_triplet(&triplet.brp, &triplet.pld, &triplet.sad);
    let findings = finding_for_session(&metrics);
    let lab_probes = lab_probes_from_triplet(&triplet.brp, &triplet.pld, &triplet.sad);
    let duration_seconds = duration_seconds(&triplet.brp)
        .min(duration_seconds(&triplet.pld))
        .min(duration_seconds(&triplet.sad));

    SessionAnalysis {
        start_date: triplet.brp.header.start_date.clone(),
        start_time: triplet.brp.header.start_time.clone(),
        duration_seconds,
        files: SessionFiles {
            brp: triplet.brp.file_name.clone(),
            pld: triplet.pld.file_name.clone(),
            sad: triplet.sad.file_name.clone(),
        },
        metrics,
        findings,
        lab_probes,
    }
}

fn lab_probes_from_triplet(
    brp: &ParsedEdf,
    pld: &ParsedEdf,
    sad: &ParsedEdf,
) -> Vec<LabProbeResult> {
    let flow = channel_contains(brp, &["flow"]);
    let trigger_cycle = channel_contains(brp, &["trig", "cyc"]);
    let pressure = channel_starts_with(pld, "press");
    let leak = channel_contains(pld, &["leak"]);
    let mask_pressure = channel_contains(pld, &["mask", "press"]);
    let spo2 = channel_contains(sad, &["spo2"]);
    let pulse = channel_contains(sad, &["pulse"]);
    let resp_rate = channel_contains(pld, &["resp", "rate"]);
    let min_vent = channel_contains(pld, &["min", "vent"]);

    vec![
        breath_morphology_probe(flow),
        trigger_cycle_synchrony_probe(flow, trigger_cycle),
        leak_pressure_probe(leak, pressure, mask_pressure),
        oximetry_coupling_probe(spo2, pulse),
        instability_windows_probe(resp_rate, min_vent),
        counterfactual_sandbox_probe(leak, pressure),
    ]
}

fn channel_contains<'a>(
    parsed: &'a ParsedEdf,
    needles: &[&str],
) -> Option<&'a crate::analysis::edf::DecodedChannel> {
    parsed.channels.iter().find(|channel| {
        let label = channel.label.to_ascii_lowercase();
        needles.iter().all(|needle| label.contains(needle))
    })
}

fn channel_starts_with<'a>(
    parsed: &'a ParsedEdf,
    needle: &str,
) -> Option<&'a crate::analysis::edf::DecodedChannel> {
    parsed
        .channels
        .iter()
        .find(|channel| channel.label.to_ascii_lowercase().starts_with(needle))
}

fn complete_triplets(parsed: Vec<ParsedEdf>) -> Vec<SessionTriplet> {
    let brps = parsed
        .iter()
        .filter(|edf| edf.role == EdfRole::Brp)
        .collect::<Vec<_>>();
    let mut triplets = Vec::new();

    for brp in brps {
        let Some(start_seconds) = timestamp_seconds(&brp.header.start_date, &brp.header.start_time)
        else {
            continue;
        };
        let Some(pld) = nearest_role(&parsed, EdfRole::Pld, start_seconds) else {
            continue;
        };
        let Some(sad) = nearest_role(&parsed, EdfRole::Sad, start_seconds) else {
            continue;
        };
        triplets.push(SessionTriplet {
            brp: brp.clone(),
            pld: pld.clone(),
            sad: sad.clone(),
        });
    }

    triplets
}

fn nearest_role(parsed: &[ParsedEdf], role: EdfRole, target_seconds: i64) -> Option<ParsedEdf> {
    parsed
        .iter()
        .filter(|edf| edf.role == role)
        .filter_map(|edf| {
            let offset = (timestamp_seconds(&edf.header.start_date, &edf.header.start_time)?
                - target_seconds)
                .abs();
            (offset <= 2).then_some((offset, edf.clone()))
        })
        .min_by_key(|(offset, _edf)| *offset)
        .map(|(_offset, edf)| edf)
}

fn duration_seconds(parsed: &ParsedEdf) -> u64 {
    if parsed.header.record_count <= 0 || parsed.header.record_duration_seconds <= 0.0 {
        return 0;
    }
    (parsed.header.record_count as f64 * parsed.header.record_duration_seconds).round() as u64
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
    use super::{analysis_from_parsed_edfs, analyze_source_paths, AnalysisStatus};
    use crate::analysis::edf::parse_edf;
    use std::path::Path;

    #[test]
    fn analyzes_complete_session_with_pressure_leak_and_unavailable_oximetry() {
        let brp = parse_edf(
            "20250816_233919_BRP.edf",
            &synthetic_edf(
                1,
                60.0,
                "16.08.25",
                "23.39.19",
                vec![SignalSpec::new(
                    "Flow.40ms",
                    "L/sec",
                    -2.0,
                    2.0,
                    -32768,
                    32767,
                    4,
                )],
                vec![vec![0, 1000, -1000, 0]],
            ),
        )
        .expect("brp parses");
        let pld = parse_edf(
            "20250816_233920_PLD.edf",
            &synthetic_edf(
                1,
                60.0,
                "16.08.25",
                "23.39.20",
                vec![
                    SignalSpec::new("Press.2s", "cmH2O", 0.0, 30.0, 0, 3000, 5),
                    SignalSpec::new("Leak.2s", "L/sec", 0.0, 1.0, 0, 100, 5),
                ],
                vec![vec![1000, 1100, 1200, 1300, 1400], vec![0, 0, 5, 10, 20]],
            ),
        )
        .expect("pld parses");
        let sad = parse_edf(
            "20250816_233920_SAD.edf",
            &synthetic_edf(
                1,
                60.0,
                "16.08.25",
                "23.39.20",
                vec![
                    SignalSpec::new("SpO2.1s", "%", -1.0, 100.0, -1, 100, 3),
                    SignalSpec::new("Pulse.1s", "bpm", -1.0, 250.0, -1, 250, 3),
                ],
                vec![vec![-1, -1, -1], vec![-1, -1, -1]],
            ),
        )
        .expect("sad parses");

        let result = analysis_from_parsed_edfs(vec![brp, pld, sad]);

        assert_eq!(result.status, AnalysisStatus::Ready);
        assert_eq!(result.sessions.len(), 1);
        let session = &result.sessions[0];
        assert_eq!(session.duration_seconds, 60);
        assert_eq!(session.files.brp, "20250816_233919_BRP.edf");
        assert_close(session.metrics.pressure.as_ref().unwrap().min, 10.0);
        assert_close(session.metrics.pressure.as_ref().unwrap().max, 14.0);
        assert_close(session.metrics.leak.as_ref().unwrap().max, 0.2);
        assert!(session.metrics.oximetry.spo2.is_none());
        assert!(session
            .metrics
            .oximetry
            .unavailable_reason
            .as_deref()
            .unwrap()
            .contains("sentinel"));
        assert_eq!(session.lab_probes.len(), 6);
        assert!(session
            .lab_probes
            .iter()
            .any(|probe| probe.id == "breath_morphology"));
        assert!(session
            .lab_probes
            .iter()
            .any(|probe| probe.id == "oximetry_coupling" && !probe.limitations.is_empty()));
        assert!(result
            .findings
            .iter()
            .any(|finding| finding.body.contains("supports discussion")));
    }

    #[test]
    fn incomplete_edfs_do_not_emit_fake_sessions() {
        let incomplete = parse_edf(
            "20250816_233920_PLD.edf",
            &synthetic_edf(
                -1,
                60.0,
                "16.08.25",
                "23.39.20",
                vec![SignalSpec::new("Press.2s", "cmH2O", 0.0, 30.0, 0, 3000, 5)],
                vec![vec![]],
            ),
        )
        .expect("header parses");

        let result = analysis_from_parsed_edfs(vec![incomplete]);

        assert_eq!(result.status, AnalysisStatus::Limited);
        assert_eq!(result.sessions.len(), 0);
        assert!(result
            .limitations
            .iter()
            .any(|limitation| limitation.contains("incomplete")));
    }

    #[test]
    #[ignore = "reads the user's Desktop CPAP fixture folder when present"]
    fn analyzes_desktop_fixture_without_inventing_oximetry() {
        let fixture = Path::new("/Users/ama/Desktop/Untitled Folder 2");
        if !fixture.exists() {
            eprintln!("Desktop fixture folder is not present; skipping");
            return;
        }

        let result = analyze_source_paths(&[fixture.to_path_buf()]).expect("analysis succeeds");

        assert_eq!(result.status, AnalysisStatus::Ready);
        assert!(result.sessions.len() >= 30);
        assert!(result
            .sessions
            .iter()
            .all(|session| session.metrics.pressure.is_some()));
        assert!(result
            .sessions
            .iter()
            .all(|session| session.metrics.leak.is_some()));
        assert!(result
            .sessions
            .iter()
            .all(|session| session.metrics.oximetry.spo2.is_none()));
        assert!(result.sessions.iter().all(|session| {
            session
                .lab_probes
                .iter()
                .any(|probe| probe.id == "leak_pressure_interaction")
        }));
        assert!(result
            .findings
            .iter()
            .any(|finding| finding.title == "Oximetry unavailable"));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {actual} to be close to {expected}"
        );
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
        start_date: &str,
        start_time: &str,
        signals: Vec<SignalSpec>,
        samples: Vec<Vec<i16>>,
    ) -> Vec<u8> {
        let signal_count = signals.len();
        let header_bytes = 256 + signal_count * 256;
        let mut header = vec![b' '; header_bytes];

        write_field(&mut header, 0, 8, "0");
        write_field(&mut header, 8, 80, "LOCAL_PATIENT");
        write_field(&mut header, 88, 80, "LOCAL_RECORDING");
        write_field(&mut header, 168, 8, start_date);
        write_field(&mut header, 176, 8, start_time);
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
                for channel_samples in &samples {
                    for sample in channel_samples {
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
