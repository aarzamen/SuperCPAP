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
    let pressure = first_stats(pld, "press");
    let leak = first_stats(pld, "leak");
    let flow = first_stats(brp, "flow");
    let spo2 = first_stats(sad, "spo2");
    let pulse = first_stats(sad, "pulse");
    let unavailable_reason = if spo2.is_none() && pulse.is_none() {
        sad.channels
            .iter()
            .find(|channel| is_label_match(channel, "spo2") || is_label_match(channel, "pulse"))
            .and_then(|channel| channel.invalid_reason.clone())
            .map(|reason| format!("SAD oximetry channels unavailable: {reason}"))
            .or_else(|| Some("No usable SAD oximetry channel was found".into()))
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
    parsed
        .channels
        .iter()
        .find(|channel| is_label_match(channel, label_needle))
        .and_then(signal_stats)
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

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "expected {actual} to be close to {expected}"
        );
    }
}
