use crate::analysis::edf::{DecodedChannel, ParsedEdf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalStats {
    pub channel: String,
    pub unit: String,
    pub samples: usize,
    pub min: f64,
    pub median: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OximetryMetrics {
    pub spo2: Option<SignalStats>,
    pub pulse: Option<SignalStats>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetrics {
    pub pressure: Option<SignalStats>,
    pub leak: Option<SignalStats>,
    pub flow: Option<SignalStats>,
    pub oximetry: OximetryMetrics,
}

pub fn metrics_from_triplet(brp: &ParsedEdf, pld: &ParsedEdf, sad: &ParsedEdf) -> SessionMetrics {
    let pressure = first_stats_by(pld, |channel| {
        channel.label.to_ascii_lowercase().starts_with("press")
    });
    let leak = first_stats(pld, "leak");
    let flow = first_stats(brp, "flow");
    let spo2 = first_oximetry_stats(sad, "spo2", 70.0, 100.0);
    let pulse = first_oximetry_stats(sad, "pulse", 25.0, 240.0);
    let unavailable_reason = if spo2.is_none() && pulse.is_none() {
        sad.channels
            .iter()
            .find(|channel| is_label_match(channel, "spo2") || is_label_match(channel, "pulse"))
            .and_then(|channel| channel.invalid_reason.clone())
            .map(|reason| format!("SAD oximetry channels unavailable: {reason}"))
            .or_else(|| {
                Some("No physiologic SAD oximetry channel was available for summary".into())
            })
    } else {
        None
    };

    SessionMetrics {
        pressure,
        leak,
        flow,
        oximetry: OximetryMetrics {
            spo2,
            pulse,
            unavailable_reason,
        },
    }
}

pub fn signal_stats(channel: &DecodedChannel) -> Option<SignalStats> {
    if channel.invalid_reason.is_some() {
        return None;
    }

    let mut values = channel
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }

    values.sort_by(|left, right| left.total_cmp(right));
    let samples = values.len();
    let min = values[0];
    let max = values[samples - 1];
    let mean = values.iter().sum::<f64>() / samples as f64;

    Some(SignalStats {
        channel: channel.label.clone(),
        unit: channel.unit.clone(),
        samples,
        min,
        median: percentile_sorted(&values, 0.50),
        p95: percentile_sorted(&values, 0.95),
        max,
        mean,
    })
}

fn first_stats(parsed: &ParsedEdf, label_needle: &str) -> Option<SignalStats> {
    first_stats_by(parsed, |channel| is_label_match(channel, label_needle))
}

fn first_stats_by(
    parsed: &ParsedEdf,
    predicate: impl Fn(&DecodedChannel) -> bool,
) -> Option<SignalStats> {
    parsed
        .channels
        .iter()
        .find(|channel| predicate(channel))
        .and_then(signal_stats)
}

fn first_oximetry_stats(
    parsed: &ParsedEdf,
    label_needle: &str,
    physiologic_min: f64,
    physiologic_max: f64,
) -> Option<SignalStats> {
    let channel = parsed
        .channels
        .iter()
        .find(|channel| is_label_match(channel, label_needle))?;
    if channel.invalid_reason.is_some() {
        return None;
    }

    let finite_count = channel
        .values
        .iter()
        .filter(|value| value.is_finite())
        .count();
    if finite_count == 0 {
        return None;
    }

    let physiologic_values = channel
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && (physiologic_min..=physiologic_max).contains(value))
        .collect::<Vec<_>>();
    let physiologic_fraction = physiologic_values.len() as f64 / finite_count as f64;
    if physiologic_values.len() < 30 || physiologic_fraction < 0.80 {
        return None;
    }

    let mut physiologic_channel = channel.clone();
    physiologic_channel.values = physiologic_values;
    signal_stats(&physiologic_channel)
}

fn is_label_match(channel: &DecodedChannel, needle: &str) -> bool {
    channel.label.to_ascii_lowercase().contains(needle)
}

fn percentile_sorted(values: &[f64], quantile: f64) -> f64 {
    let index = ((values.len() - 1) as f64 * quantile).round() as usize;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::signal_stats;
    use crate::analysis::edf::DecodedChannel;

    #[test]
    fn computes_signal_stats_from_decoded_physical_values() {
        let channel = channel("Press.2s", "cmH2O", vec![10.0, 12.0, 14.0, 16.0, 18.0]);

        let stats = signal_stats(&channel).expect("stats available");

        assert_eq!(stats.channel, "Press.2s");
        assert_eq!(stats.unit, "cmH2O");
        assert_eq!(stats.samples, 5);
        assert_close(stats.min, 10.0);
        assert_close(stats.median, 14.0);
        assert_close(stats.p95, 18.0);
        assert_close(stats.max, 18.0);
        assert_close(stats.mean, 14.0);
    }

    #[test]
    fn refuses_stats_for_invalid_or_empty_channels() {
        let mut invalid = channel("SpO2.1s", "%", vec![-1.0, -1.0]);
        invalid.invalid_reason = Some("oximetry channel contains sentinel values only".into());
        let empty = channel("Leak.2s", "L/sec", vec![]);

        assert_eq!(signal_stats(&invalid), None);
        assert_eq!(signal_stats(&empty), None);
    }

    #[test]
    fn session_metrics_do_not_treat_mixed_sentinel_oximetry_as_valid() {
        let brp = parsed(
            "sample_BRP.edf",
            crate::analysis::edf::EdfRole::Brp,
            vec![channel("Flow.40ms", "L/sec", vec![0.1, -0.1, 0.2, -0.2])],
        );
        let pld = parsed(
            "sample_PLD.edf",
            crate::analysis::edf::EdfRole::Pld,
            vec![channel("Press.2s", "cmH2O", vec![10.0, 10.2, 10.4])],
        );
        let sad = parsed(
            "sample_SAD.edf",
            crate::analysis::edf::EdfRole::Sad,
            vec![
                channel("SpO2.1s", "%", vec![-1.0, -1.0, 97.0, -1.0, -1.0]),
                channel("Pulse.1s", "bpm", vec![-1.0, -1.0, 70.0, -1.0, -1.0]),
            ],
        );

        let metrics = super::metrics_from_triplet(&brp, &pld, &sad);

        assert!(metrics.oximetry.spo2.is_none());
        assert!(metrics.oximetry.pulse.is_none());
        assert!(metrics
            .oximetry
            .unavailable_reason
            .as_deref()
            .unwrap()
            .contains("physiologic"));
    }

    #[test]
    fn pressure_metric_prefers_press_channel_over_maskpress() {
        let brp = parsed(
            "sample_BRP.edf",
            crate::analysis::edf::EdfRole::Brp,
            vec![channel("Flow.40ms", "L/sec", vec![0.1, -0.1])],
        );
        let pld = parsed(
            "sample_PLD.edf",
            crate::analysis::edf::EdfRole::Pld,
            vec![
                channel("MaskPress.2s", "cmH2O", vec![7.0, 7.0, 7.0]),
                channel("Press.2s", "cmH2O", vec![11.0, 12.0, 13.0]),
            ],
        );
        let sad = parsed("sample_SAD.edf", crate::analysis::edf::EdfRole::Sad, vec![]);

        let metrics = super::metrics_from_triplet(&brp, &pld, &sad);

        let pressure = metrics.pressure.expect("pressure stats");
        assert_eq!(pressure.channel, "Press.2s");
        assert_close(pressure.median, 12.0);
    }

    fn channel(label: &str, unit: &str, values: Vec<f64>) -> DecodedChannel {
        DecodedChannel {
            label: label.into(),
            unit: unit.into(),
            physical_min: 0.0,
            physical_max: 0.0,
            digital_min: 0,
            digital_max: 0,
            samples_per_record: values.len(),
            sample_rate_hz: 1.0,
            values,
            invalid_reason: None,
        }
    }

    fn parsed(
        file_name: &str,
        role: crate::analysis::edf::EdfRole,
        channels: Vec<DecodedChannel>,
    ) -> crate::analysis::edf::ParsedEdf {
        crate::analysis::edf::ParsedEdf {
            file_name: file_name.into(),
            role,
            valid: true,
            limited: false,
            header: crate::analysis::edf::EdfHeader {
                version: "0".into(),
                header_bytes: 256,
                record_count: 1,
                record_duration_seconds: 60.0,
                signal_count: channels.len(),
                start_date: "01.01.25".into(),
                start_time: "00.00.00".into(),
            },
            channels,
            warnings: Vec::new(),
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {actual} to be close to {expected}"
        );
    }
}
