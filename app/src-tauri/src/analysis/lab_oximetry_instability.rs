use crate::analysis::edf::DecodedChannel;
use crate::analysis::lab_common::LabProbeResult;

const OXIMETRY_ID: &str = "oximetry_coupling";
const OXIMETRY_TITLE: &str = "Oximetry coupling";
const INSTABILITY_ID: &str = "instability_windows";
const INSTABILITY_TITLE: &str = "Instability windows";

pub fn oximetry_coupling_probe(
    spo2: Option<&DecodedChannel>,
    pulse: Option<&DecodedChannel>,
) -> LabProbeResult {
    let Some(spo2) = spo2 else {
        return LabProbeResult::gated(
            OXIMETRY_ID,
            OXIMETRY_TITLE,
            "Oximetry coupling is gated until decoded SAD SpO2.1s is available.",
            vec!["Valid SAD SpO2.1s and Pulse.1s are required for this exploratory probe.".into()],
        );
    };
    let Some(pulse) = pulse else {
        return LabProbeResult::gated(
            OXIMETRY_ID,
            OXIMETRY_TITLE,
            "Oximetry coupling is gated until decoded SAD Pulse.1s is available.",
            vec!["Valid SAD SpO2.1s and Pulse.1s are required for this exploratory probe.".into()],
        );
    };

    if let Some(reason) = &spo2.invalid_reason {
        return gated_oximetry(format!("SpO2.1s is unavailable: {reason}"));
    }
    if let Some(reason) = &pulse.invalid_reason {
        return gated_oximetry(format!("Pulse.1s is unavailable: {reason}"));
    }

    let pairs = paired_oximetry_samples(spo2, pulse);
    if pairs.len() < 30 {
        return gated_oximetry(
            "Too few physiologic SpO2/Pulse sample pairs were available after sentinel and range checks."
                .into(),
        );
    }

    let drop_events = spo2_drop_events(&pairs);
    let responses = pulse_responses(&pairs, &drop_events);
    let mean_response = mean(&responses).unwrap_or(0.0);
    let direction = if responses.is_empty() {
        "insufficient paired pulse response"
    } else if mean_response >= 3.0 {
        "pulse tended to rise after drops"
    } else if mean_response <= -3.0 {
        "pulse tended to fall after drops"
    } else {
        "pulse response was small or mixed"
    };

    let summary = if drop_events.is_empty() {
        "Exploratory oximetry coupling found no qualifying SpO2 drops in the usable decoded window."
    } else {
        "Exploratory oximetry coupling found qualifying SpO2 drops and summarizes nearby pulse movement."
    };

    let mut evidence = vec![
        format!("Usable paired oximetry samples: {}", pairs.len()),
        format!("SpO2 drop count: {}", drop_events.len()),
        format!("Pulse response summary: {direction}"),
    ];
    if !responses.is_empty() {
        evidence.push(format!(
            "Mean pulse change within 30 s after drops: {:.1} bpm",
            mean_response
        ));
    }

    LabProbeResult::available(OXIMETRY_ID, OXIMETRY_TITLE, summary, evidence)
}

pub fn instability_windows_probe(
    resp_rate: Option<&DecodedChannel>,
    min_vent: Option<&DecodedChannel>,
) -> LabProbeResult {
    let resp_values = usable_series(resp_rate, 3.0, 80.0);
    let vent_values = usable_series(min_vent, 0.1, 80.0);

    if resp_values.is_empty() && vent_values.is_empty() {
        return LabProbeResult::gated(
            INSTABILITY_ID,
            INSTABILITY_TITLE,
            "Instability windows are gated until decoded PLD respiratory-rate or minute-ventilation samples are available.",
            vec![
                "PLD RespRate.2s and/or MinVent.2s are required for respiratory variability windowing."
                    .into(),
            ],
        );
    }

    let mut evidence = Vec::new();
    let mut limitations = vec![
        "Exploratory variability window only; this does not infer arousal state or prescribe device settings."
            .into(),
    ];
    let mut candidate_count = 0usize;

    if let Some(metric) = variability_metric("RespRate.2s", &resp_values) {
        candidate_count += metric.candidate_windows;
        evidence.push(metric.evidence_line);
    } else if resp_rate.is_some() {
        limitations
            .push("RespRate.2s had too few physiologic samples for windowed variability.".into());
    } else {
        limitations.push("RespRate.2s was not available for this probe.".into());
    }

    if let Some(metric) = variability_metric("MinVent.2s", &vent_values) {
        candidate_count += metric.candidate_windows;
        evidence.push(metric.evidence_line);
    } else if min_vent.is_some() {
        limitations
            .push("MinVent.2s had too few physiologic samples for windowed variability.".into());
    } else {
        limitations.push("MinVent.2s was not available for this probe.".into());
    }

    if evidence.is_empty() {
        return LabProbeResult::gated(
            INSTABILITY_ID,
            INSTABILITY_TITLE,
            "Instability windows are gated because the supplied PLD channels were sparse or non-physiologic.",
            limitations,
        );
    }

    let summary = if candidate_count == 0 {
        "Exploratory instability windowing found no high-variability candidate windows in the decoded PLD signals."
    } else {
        "Exploratory instability windowing found high-variability respiratory signal windows that deserve review."
    };

    if resp_values.is_empty() || vent_values.is_empty() {
        LabProbeResult::limited(
            INSTABILITY_ID,
            INSTABILITY_TITLE,
            summary,
            evidence,
            limitations,
        )
    } else {
        LabProbeResult::available(INSTABILITY_ID, INSTABILITY_TITLE, summary, evidence)
    }
}

fn gated_oximetry(limitation: String) -> LabProbeResult {
    LabProbeResult::gated(
        OXIMETRY_ID,
        OXIMETRY_TITLE,
        "Oximetry coupling is gated because SAD oximetry is not physiologic enough to interpret.",
        vec![limitation],
    )
}

fn paired_oximetry_samples(spo2: &DecodedChannel, pulse: &DecodedChannel) -> Vec<(f64, f64)> {
    spo2.values
        .iter()
        .zip(pulse.values.iter())
        .filter_map(|(spo2_value, pulse_value)| {
            let spo2_ok = spo2_value.is_finite() && (70.0..=100.0).contains(spo2_value);
            let pulse_ok = pulse_value.is_finite() && (25.0..=240.0).contains(pulse_value);
            if spo2_ok && pulse_ok {
                Some((*spo2_value, *pulse_value))
            } else {
                None
            }
        })
        .collect()
}

fn spo2_drop_events(pairs: &[(f64, f64)]) -> Vec<usize> {
    let mut events = Vec::new();
    let mut last_event_index = None;

    for index in 30..pairs.len() {
        let baseline = pairs[index - 30..index]
            .iter()
            .map(|(spo2, _pulse)| *spo2)
            .fold(f64::NEG_INFINITY, f64::max);
        let drop = baseline - pairs[index].0;
        let separated = last_event_index
            .map(|previous| index.saturating_sub(previous) >= 30)
            .unwrap_or(true);
        if drop >= 3.0 && separated {
            events.push(index);
            last_event_index = Some(index);
        }
    }

    events
}

fn pulse_responses(pairs: &[(f64, f64)], drop_events: &[usize]) -> Vec<f64> {
    drop_events
        .iter()
        .filter_map(|event_index| {
            let baseline_start = event_index.saturating_sub(10);
            let baseline = mean(
                &pairs[baseline_start..*event_index]
                    .iter()
                    .map(|(_spo2, pulse)| *pulse)
                    .collect::<Vec<_>>(),
            )?;
            let response_end = (*event_index + 30).min(pairs.len());
            let response_peak = pairs[*event_index..response_end]
                .iter()
                .map(|(_spo2, pulse)| *pulse)
                .fold(f64::NEG_INFINITY, f64::max);
            Some(response_peak - baseline)
        })
        .collect()
}

fn usable_series(channel: Option<&DecodedChannel>, min: f64, max: f64) -> Vec<f64> {
    let Some(channel) = channel else {
        return Vec::new();
    };
    if channel.invalid_reason.is_some() {
        return Vec::new();
    }

    channel
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite() && (min..=max).contains(value))
        .collect()
}

struct VariabilityMetric {
    candidate_windows: usize,
    evidence_line: String,
}

fn variability_metric(label: &str, values: &[f64]) -> Option<VariabilityMetric> {
    const WINDOW_SAMPLES: usize = 30;
    const STEP_SAMPLES: usize = 15;

    if values.len() < WINDOW_SAMPLES {
        return None;
    }

    let mut coefficients = Vec::new();
    let mut start = 0usize;
    while start + WINDOW_SAMPLES <= values.len() {
        let window = &values[start..start + WINDOW_SAMPLES];
        let avg = mean(window)?;
        if avg.abs() > f64::EPSILON {
            coefficients.push(std_dev(window, avg) / avg.abs());
        }
        start += STEP_SAMPLES;
    }

    if coefficients.is_empty() {
        return None;
    }

    let candidate_windows = coefficients
        .iter()
        .filter(|coefficient| **coefficient >= 0.20)
        .count();
    let median_cv = percentile(coefficients.clone(), 0.50);
    let max_cv = coefficients.iter().copied().fold(0.0, f64::max);

    Some(VariabilityMetric {
        candidate_windows,
        evidence_line: format!(
            "{label} variability windows: {candidate_windows} candidates, median CV {:.2}, max CV {:.2}",
            median_cv, max_cv
        ),
    })
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn std_dev(values: &[f64], avg: f64) -> f64 {
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - avg;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn percentile(mut values: Vec<f64>, fraction: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let index = ((values.len().saturating_sub(1)) as f64 * fraction).round() as usize;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::{instability_windows_probe, oximetry_coupling_probe};
    use crate::analysis::edf::DecodedChannel;
    use crate::analysis::lab_common::LabProbeStatus;

    #[test]
    fn oximetry_gates_when_channels_are_missing_invalid_or_sentinel_only() {
        let missing = oximetry_coupling_probe(None, None);
        assert_eq!(missing.id, "oximetry_coupling");
        assert_eq!(missing.status, LabProbeStatus::Gated);
        assert!(missing
            .limitations
            .iter()
            .any(|line| line.contains("SpO2.1s") && line.contains("Pulse.1s")));

        let mut invalid_spo2 = channel("SpO2.1s", "%", vec![96.0; 60]);
        invalid_spo2.invalid_reason = Some("sentinel values only".into());
        let pulse = channel("Pulse.1s", "bpm", vec![70.0; 60]);
        let invalid = oximetry_coupling_probe(Some(&invalid_spo2), Some(&pulse));
        assert_eq!(invalid.status, LabProbeStatus::Gated);
        assert!(invalid
            .limitations
            .iter()
            .any(|line| line.contains("sentinel values only")));

        let sentinel_spo2 = channel("SpO2.1s", "%", vec![0.0; 60]);
        let sentinel_pulse = channel("Pulse.1s", "bpm", vec![0.0; 60]);
        let sentinel = oximetry_coupling_probe(Some(&sentinel_spo2), Some(&sentinel_pulse));
        assert_eq!(sentinel.status, LabProbeStatus::Gated);
        assert!(sentinel.summary.contains("not physiologic"));
    }

    #[test]
    fn oximetry_reports_spo2_drops_and_pulse_response_from_decoded_values() {
        let mut spo2_values = vec![97.0; 130];
        for value in &mut spo2_values[40..50] {
            *value = 92.0;
        }
        for value in &mut spo2_values[90..100] {
            *value = 93.0;
        }

        let mut pulse_values = vec![68.0; 130];
        for value in &mut pulse_values[40..70] {
            *value = 78.0;
        }
        for value in &mut pulse_values[90..120] {
            *value = 74.0;
        }

        let result = oximetry_coupling_probe(
            Some(&channel("SpO2.1s", "%", spo2_values)),
            Some(&channel("Pulse.1s", "bpm", pulse_values)),
        );

        assert_eq!(result.status, LabProbeStatus::Available);
        assert!(result.summary.contains("Exploratory"));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("not a device-setting recommendation")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "SpO2 drop count: 2"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Pulse response summary: pulse tended to rise after drops"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Mean pulse change within 30 s after drops: 8.0 bpm"));
    }

    #[test]
    fn instability_windows_gate_when_no_respiratory_variability_channels_exist() {
        let result = instability_windows_probe(None, None);

        assert_eq!(result.id, "instability_windows");
        assert_eq!(result.status, LabProbeStatus::Gated);
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("RespRate.2s") && line.contains("MinVent.2s")));
    }

    #[test]
    fn instability_windows_are_limited_with_one_usable_pld_channel() {
        let resp_rate = channel("RespRate.2s", "breaths/min", vec![14.0; 90]);

        let result = instability_windows_probe(Some(&resp_rate), None);

        assert_eq!(result.status, LabProbeStatus::Limited);
        assert!(result.evidence.iter().any(|line| line
            == "RespRate.2s variability windows: 0 candidates, median CV 0.00, max CV 0.00"));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("MinVent.2s was not available")));
    }

    #[test]
    fn instability_windows_report_candidate_variability_from_resp_rate_and_minute_ventilation() {
        let mut resp_rate_values = vec![14.0; 120];
        for (index, value) in resp_rate_values[45..75].iter_mut().enumerate() {
            *value = if index % 2 == 0 { 8.0 } else { 24.0 };
        }

        let mut min_vent_values = vec![6.0; 120];
        for (index, value) in min_vent_values[60..90].iter_mut().enumerate() {
            *value = if index % 2 == 0 { 3.0 } else { 10.0 };
        }

        let result = instability_windows_probe(
            Some(&channel("RespRate.2s", "breaths/min", resp_rate_values)),
            Some(&channel("MinVent.2s", "L/min", min_vent_values)),
        );

        assert_eq!(result.status, LabProbeStatus::Available);
        assert!(result.summary.contains("Exploratory"));
        assert!(result.summary.contains("deserve review"));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("not a device-setting recommendation")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.starts_with("RespRate.2s variability windows:")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.starts_with("MinVent.2s variability windows:")));
    }

    fn channel(label: &str, unit: &str, values: Vec<f64>) -> DecodedChannel {
        DecodedChannel {
            label: label.into(),
            unit: unit.into(),
            physical_min: values.iter().copied().fold(0.0, f64::min),
            physical_max: values.iter().copied().fold(0.0, f64::max),
            digital_min: -32768,
            digital_max: 32767,
            samples_per_record: 1,
            sample_rate_hz: 1.0,
            values,
            invalid_reason: None,
        }
    }
}
