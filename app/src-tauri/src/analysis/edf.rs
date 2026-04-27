use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdfRole {
    Brp,
    Pld,
    Sad,
    Eve,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdfHeader {
    pub version: String,
    pub header_bytes: usize,
    pub record_count: i64,
    pub record_duration_seconds: f64,
    pub signal_count: usize,
    pub start_date: String,
    pub start_time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedChannel {
    pub label: String,
    pub unit: String,
    pub physical_min: f64,
    pub physical_max: f64,
    pub digital_min: i16,
    pub digital_max: i16,
    pub samples_per_record: usize,
    pub sample_rate_hz: f64,
    pub values: Vec<f64>,
    pub invalid_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedEdf {
    pub file_name: String,
    pub role: EdfRole,
    pub valid: bool,
    pub limited: bool,
    pub header: EdfHeader,
    pub channels: Vec<DecodedChannel>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdfError {
    message: String,
}

impl EdfError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EdfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for EdfError {}

pub fn parse_edf(file_name: &str, bytes: &[u8]) -> Result<ParsedEdf, EdfError> {
    if bytes.len() < 256 {
        return Err(EdfError::new("EDF is shorter than the fixed header"));
    }

    let signal_count = parse_usize(field(bytes, 252, 4)?, "signal count")?;
    let header_bytes = parse_usize(field(bytes, 184, 8)?, "header byte count")?;
    if header_bytes < 256 + signal_count * 256 {
        return Err(EdfError::new(
            "EDF header byte count is too small for signal metadata",
        ));
    }
    if bytes.len() < header_bytes {
        return Err(EdfError::new("EDF is shorter than its declared header"));
    }

    let record_count = parse_i64(field(bytes, 236, 8)?, "record count")?;
    let record_duration_seconds = parse_f64(field(bytes, 244, 8)?, "record duration")?;
    let header = EdfHeader {
        version: field(bytes, 0, 8)?,
        header_bytes,
        record_count,
        record_duration_seconds,
        signal_count,
        start_date: field(bytes, 168, 8)?,
        start_time: field(bytes, 176, 8)?,
    };

    let metadata = parse_signal_metadata(bytes, signal_count, record_duration_seconds)?;
    let samples_per_record_total: usize = metadata
        .iter()
        .map(|channel| channel.samples_per_record)
        .sum();
    let expected_data_bytes = if record_count > 0 {
        record_count as usize * samples_per_record_total * 2
    } else {
        0
    };
    let available_data_bytes = bytes.len().saturating_sub(header_bytes);

    let mut warnings = Vec::new();
    let mut valid = true;
    let mut limited = false;
    let mut channels = metadata;

    if record_count <= 0 {
        valid = false;
        limited = true;
        warnings
            .push("EDF is header-only or incomplete because record count is not positive".into());
    } else if available_data_bytes < expected_data_bytes {
        valid = false;
        limited = true;
        warnings.push(format!(
            "EDF is incomplete: expected {expected_data_bytes} data bytes but found {available_data_bytes}"
        ));
    } else {
        decode_samples(bytes, header_bytes, record_count as usize, &mut channels)?;
        mark_invalid_sentinel_channels(&mut channels);
    }

    Ok(ParsedEdf {
        file_name: file_name.to_string(),
        role: role_from_file_name(file_name),
        valid,
        limited,
        header,
        channels,
        warnings,
    })
}

fn parse_signal_metadata(
    bytes: &[u8],
    signal_count: usize,
    record_duration_seconds: f64,
) -> Result<Vec<DecodedChannel>, EdfError> {
    let mut offset = 256;
    let labels = parse_string_array(bytes, &mut offset, signal_count, 16)?;
    let _transducers = parse_string_array(bytes, &mut offset, signal_count, 80)?;
    let units = parse_string_array(bytes, &mut offset, signal_count, 8)?;
    let physical_mins = parse_f64_array(bytes, &mut offset, signal_count, 8, "physical min")?;
    let physical_maxes = parse_f64_array(bytes, &mut offset, signal_count, 8, "physical max")?;
    let digital_mins = parse_i16_array(bytes, &mut offset, signal_count, 8, "digital min")?;
    let digital_maxes = parse_i16_array(bytes, &mut offset, signal_count, 8, "digital max")?;
    let _prefilters = parse_string_array(bytes, &mut offset, signal_count, 80)?;
    let samples_per_record =
        parse_usize_array(bytes, &mut offset, signal_count, 8, "samples per record")?;
    let _reserved = parse_string_array(bytes, &mut offset, signal_count, 32)?;

    let channels = (0..signal_count)
        .map(|index| {
            let sample_rate_hz = if record_duration_seconds > 0.0 {
                samples_per_record[index] as f64 / record_duration_seconds
            } else {
                0.0
            };

            DecodedChannel {
                label: labels[index].clone(),
                unit: units[index].clone(),
                physical_min: physical_mins[index],
                physical_max: physical_maxes[index],
                digital_min: digital_mins[index],
                digital_max: digital_maxes[index],
                samples_per_record: samples_per_record[index],
                sample_rate_hz,
                values: Vec::new(),
                invalid_reason: None,
            }
        })
        .collect();

    Ok(channels)
}

fn decode_samples(
    bytes: &[u8],
    header_bytes: usize,
    record_count: usize,
    channels: &mut [DecodedChannel],
) -> Result<(), EdfError> {
    let samples_per_record_total: usize = channels
        .iter()
        .map(|channel| channel.samples_per_record)
        .sum();
    let bytes_per_record = samples_per_record_total * 2;

    for channel in channels.iter_mut() {
        channel
            .values
            .reserve(record_count * channel.samples_per_record);
    }

    for record_index in 0..record_count {
        let mut offset = header_bytes + record_index * bytes_per_record;
        for channel in channels.iter_mut() {
            for _sample_index in 0..channel.samples_per_record {
                let raw = read_i16_le(bytes, offset)?;
                offset += 2;
                channel.values.push(scale_sample(raw, channel));
            }
        }
    }

    Ok(())
}

fn scale_sample(digital: i16, channel: &DecodedChannel) -> f64 {
    let digital_span = channel.digital_max as f64 - channel.digital_min as f64;
    if digital_span == 0.0 {
        return channel.physical_min;
    }

    channel.physical_min
        + (digital as f64 - channel.digital_min as f64)
            * (channel.physical_max - channel.physical_min)
            / digital_span
}

fn mark_invalid_sentinel_channels(channels: &mut [DecodedChannel]) {
    for channel in channels {
        let label = channel.label.to_ascii_lowercase();
        let is_oximetry = label.contains("spo2") || label.contains("pulse");
        if !is_oximetry || channel.values.is_empty() {
            continue;
        }

        let sentinel_only = channel
            .values
            .iter()
            .all(|value| value.is_nan() || *value <= 0.0);
        if sentinel_only {
            channel.invalid_reason = Some("oximetry channel contains sentinel values only".into());
        }
    }
}

fn role_from_file_name(file_name: &str) -> EdfRole {
    let upper = file_name.to_ascii_uppercase();
    if upper.contains("_BRP.") {
        EdfRole::Brp
    } else if upper.contains("_PLD.") {
        EdfRole::Pld
    } else if upper.contains("_SAD.") {
        EdfRole::Sad
    } else if upper.contains("_EVE.") {
        EdfRole::Eve
    } else {
        EdfRole::Unknown
    }
}

fn parse_string_array(
    bytes: &[u8],
    offset: &mut usize,
    count: usize,
    width: usize,
) -> Result<Vec<String>, EdfError> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(field(bytes, *offset, width)?);
        *offset += width;
    }
    Ok(values)
}

fn parse_f64_array(
    bytes: &[u8],
    offset: &mut usize,
    count: usize,
    width: usize,
    name: &str,
) -> Result<Vec<f64>, EdfError> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(parse_f64(field(bytes, *offset, width)?, name)?);
        *offset += width;
    }
    Ok(values)
}

fn parse_i16_array(
    bytes: &[u8],
    offset: &mut usize,
    count: usize,
    width: usize,
    name: &str,
) -> Result<Vec<i16>, EdfError> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(parse_i16(field(bytes, *offset, width)?, name)?);
        *offset += width;
    }
    Ok(values)
}

fn parse_usize_array(
    bytes: &[u8],
    offset: &mut usize,
    count: usize,
    width: usize,
    name: &str,
) -> Result<Vec<usize>, EdfError> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(parse_usize(field(bytes, *offset, width)?, name)?);
        *offset += width;
    }
    Ok(values)
}

fn field(bytes: &[u8], start: usize, width: usize) -> Result<String, EdfError> {
    let end = start + width;
    if end > bytes.len() {
        return Err(EdfError::new("EDF field extends past available bytes"));
    }
    Ok(String::from_utf8_lossy(&bytes[start..end])
        .trim()
        .to_string())
}

fn read_i16_le(bytes: &[u8], offset: usize) -> Result<i16, EdfError> {
    let end = offset + 2;
    if end > bytes.len() {
        return Err(EdfError::new("EDF sample extends past available bytes"));
    }
    Ok(i16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn parse_f64(value: String, name: &str) -> Result<f64, EdfError> {
    value
        .parse::<f64>()
        .map_err(|_| EdfError::new(format!("Invalid EDF {name}: {value}")))
}

fn parse_i64(value: String, name: &str) -> Result<i64, EdfError> {
    value
        .parse::<i64>()
        .map_err(|_| EdfError::new(format!("Invalid EDF {name}: {value}")))
}

fn parse_i16(value: String, name: &str) -> Result<i16, EdfError> {
    value
        .parse::<i16>()
        .map_err(|_| EdfError::new(format!("Invalid EDF {name}: {value}")))
}

fn parse_usize(value: String, name: &str) -> Result<usize, EdfError> {
    value
        .parse::<usize>()
        .map_err(|_| EdfError::new(format!("Invalid EDF {name}: {value}")))
}

#[cfg(test)]
mod tests {
    use super::{parse_edf, EdfRole};

    #[test]
    fn parses_fixed_header_and_signal_metadata() {
        let bytes = synthetic_edf(
            1,
            2.0,
            vec![
                SignalSpec::new("Flow.40ms", "L/sec", -2.0, 2.0, -32768, 32767, 50),
                SignalSpec::new("Press.2s", "cmH2O", 0.0, 30.0, 0, 3000, 1),
            ],
            vec![vec![0; 50], vec![1500]],
        );

        let parsed = parse_edf("20250914_211945_BRP.edf", &bytes).expect("synthetic EDF parses");

        assert_eq!(parsed.file_name, "20250914_211945_BRP.edf");
        assert_eq!(parsed.role, EdfRole::Brp);
        assert!(parsed.valid);
        assert!(!parsed.limited);
        assert_eq!(parsed.header.record_count, 1);
        assert_eq!(parsed.header.record_duration_seconds, 2.0);
        assert_eq!(parsed.channels.len(), 2);

        let flow = &parsed.channels[0];
        assert_eq!(flow.label, "Flow.40ms");
        assert_eq!(flow.unit, "L/sec");
        assert_eq!(flow.samples_per_record, 50);
        assert_eq!(flow.sample_rate_hz, 25.0);
        assert_eq!(flow.digital_min, -32768);
        assert_eq!(flow.digital_max, 32767);

        let pressure = &parsed.channels[1];
        assert_eq!(pressure.label, "Press.2s");
        assert_eq!(pressure.unit, "cmH2O");
        assert_eq!(pressure.samples_per_record, 1);
        assert_eq!(pressure.sample_rate_hz, 0.5);
    }

    #[test]
    fn decodes_record_ordered_i16_samples_and_scales_to_physical_values() {
        let bytes = synthetic_edf(
            1,
            1.0,
            vec![SignalSpec::new("Leak.2s", "L/sec", 0.0, 100.0, 0, 10, 3)],
            vec![vec![0, 5, 10]],
        );

        let parsed = parse_edf("20250914_211945_PLD.edf", &bytes).expect("synthetic EDF parses");
        let leak = &parsed.channels[0];

        assert_eq!(parsed.role, EdfRole::Pld);
        assert_eq!(leak.values.len(), 3);
        assert_close(leak.values[0], 0.0);
        assert_close(leak.values[1], 50.0);
        assert_close(leak.values[2], 100.0);
    }

    #[test]
    fn marks_header_only_or_incomplete_edf_as_limited_without_fake_samples() {
        let mut bytes = synthetic_edf(
            -1,
            60.0,
            vec![SignalSpec::new("SpO2.1s", "%", 0.0, 100.0, 0, 100, 60)],
            vec![vec![]],
        );
        bytes.truncate(512);

        let parsed = parse_edf("20250914_205646_SAD.edf", &bytes).expect("header still parses");

        assert_eq!(parsed.role, EdfRole::Sad);
        assert!(!parsed.valid);
        assert!(parsed.limited);
        assert_eq!(parsed.channels[0].values.len(), 0);
        assert!(parsed
            .warnings
            .iter()
            .any(|warning| warning.contains("header-only") || warning.contains("incomplete")));
    }

    #[test]
    fn marks_sad_oximetry_sentinel_channels_unavailable() {
        let bytes = synthetic_edf(
            1,
            1.0,
            vec![SignalSpec::new("SpO2.1s", "%", -1.0, 100.0, -1, 100, 3)],
            vec![vec![-1, -1, -1]],
        );

        let parsed = parse_edf("20250914_211945_SAD.edf", &bytes).expect("synthetic EDF parses");

        assert_eq!(
            parsed.channels[0].invalid_reason.as_deref(),
            Some("oximetry channel contains sentinel values only")
        );
    }

    #[test]
    #[ignore = "reads local CPAP sample files outside the repository"]
    fn parses_known_local_resmed_sample_ranges() {
        let sample_dir = std::path::Path::new(
            "/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914",
        );
        if !sample_dir.exists() {
            eprintln!("local CPAP sample directory not present; skipping");
            return;
        }

        let brp = parse_local_sample(sample_dir, "20250914_211945_BRP.edf");
        let pld = parse_local_sample(sample_dir, "20250914_211945_PLD.edf");
        let sad = parse_local_sample(sample_dir, "20250914_211945_SAD.edf");

        assert_eq!(brp.role, EdfRole::Brp);
        assert_eq!(brp.header.record_count, 16);
        assert_eq!(brp.header.record_duration_seconds, 60.0);
        assert_eq!(channel(&brp, "Flow.40ms").values.len(), 24_000);
        assert_eq!(channel(&brp, "Press.40ms").sample_rate_hz, 25.0);

        assert_eq!(pld.role, EdfRole::Pld);
        assert_eq!(pld.header.record_count, 16);
        assert_eq!(pld.header.record_duration_seconds, 60.0);
        let pressure = channel(&pld, "Press.2s");
        let (pressure_min, pressure_max) = min_max(&pressure.values);
        assert_close(pressure_min, 15.0);
        assert_close(pressure_max, 15.0);

        let leak = channel(&pld, "Leak.2s");
        let (_leak_min, leak_max) = min_max(&leak.values);
        assert!(
            percentile(leak.values.clone(), 0.95) < 0.01,
            "expected leak p95 near 0 L/s"
        );
        assert!(
            (0.33..=0.35).contains(&leak_max),
            "expected leak max near 0.340 L/s, got {leak_max}"
        );

        assert_eq!(sad.role, EdfRole::Sad);
        assert_eq!(
            channel(&sad, "SpO2.1s").invalid_reason.as_deref(),
            Some("oximetry channel contains sentinel values only")
        );
        assert_eq!(
            channel(&sad, "Pulse.1s").invalid_reason.as_deref(),
            Some("oximetry channel contains sentinel values only")
        );
    }

    #[test]
    #[ignore = "reads local CPAP sample files outside the repository"]
    fn marks_known_local_incomplete_files_limited() {
        let sample_dir = std::path::Path::new(
            "/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914",
        );
        if !sample_dir.exists() {
            eprintln!("local CPAP sample directory not present; skipping");
            return;
        }

        let incomplete = parse_local_sample(sample_dir, "20250914_205646_BRP.edf");

        assert!(!incomplete.valid);
        assert!(incomplete.limited);
        assert!(incomplete
            .channels
            .iter()
            .all(|channel| channel.values.is_empty()));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {actual} to be close to {expected}"
        );
    }

    fn parse_local_sample(sample_dir: &std::path::Path, file_name: &str) -> super::ParsedEdf {
        let path = sample_dir.join(file_name);
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        });
        parse_edf(file_name, &bytes).unwrap_or_else(|error| {
            panic!("failed to parse {}: {error}", path.display());
        })
    }

    fn channel<'a>(parsed: &'a super::ParsedEdf, label: &str) -> &'a super::DecodedChannel {
        parsed
            .channels
            .iter()
            .find(|channel| channel.label == label)
            .unwrap_or_else(|| panic!("missing channel {label} in {}", parsed.file_name))
    }

    fn min_max(values: &[f64]) -> (f64, f64) {
        assert!(!values.is_empty(), "expected non-empty values");
        values
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                (min.min(*value), max.max(*value))
            })
    }

    fn percentile(mut values: Vec<f64>, quantile: f64) -> f64 {
        assert!(!values.is_empty(), "expected non-empty values");
        values.sort_by(|left, right| left.total_cmp(right));
        let index = ((values.len() - 1) as f64 * quantile).round() as usize;
        values[index]
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
        samples: Vec<Vec<i16>>,
    ) -> Vec<u8> {
        let signal_count = signals.len();
        let header_bytes = 256 + signal_count * 256;
        let mut header = vec![b' '; header_bytes];

        write_field(&mut header, 0, 8, "0");
        write_field(&mut header, 8, 80, "LOCAL_PATIENT");
        write_field(&mut header, 88, 80, "LOCAL_RECORDING");
        write_field(&mut header, 168, 8, "14.09.25");
        write_field(&mut header, 176, 8, "21.19.45");
        write_field(&mut header, 184, 8, &header_bytes.to_string());
        write_field(&mut header, 236, 8, &records.to_string());
        write_field(
            &mut header,
            244,
            8,
            &format_record_duration(record_duration_seconds),
        );
        write_field(&mut header, 252, 4, &signal_count.to_string());

        let mut offset = 256;
        for signal in &signals {
            write_field(&mut header, offset, 16, signal.label);
            offset += 16;
        }
        for _ in &signals {
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
        for _ in &signals {
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
        for _ in &signals {
            write_field(&mut header, offset, 32, "");
            offset += 32;
        }

        let mut bytes = header;
        if records > 0 {
            for _record_index in 0..records {
                for signal_samples in &samples {
                    for sample in signal_samples {
                        bytes.extend_from_slice(&sample.to_le_bytes());
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

    fn format_record_duration(value: f64) -> String {
        if value.fract() == 0.0 {
            format!("{}", value as i64)
        } else {
            value.to_string()
        }
    }
}
