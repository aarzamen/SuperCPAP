use crate::analysis::edf::DecodedChannel;
use crate::analysis::lab_common::{LabProbeResult, LabProbeStatus};

const PROBE_ID: &str = "breath_morphology";
const PROBE_TITLE: &str = "Breath morphology";
const MIN_COMPLETE_BREATHS: usize = 3;
const AVAILABLE_BREATHS: usize = 5;

#[derive(Debug, Clone)]
struct BreathSegment {
    duration_seconds: f64,
    inspiratory_seconds: f64,
    expiratory_seconds: f64,
    inspiratory_peak: f64,
    expiratory_peak: f64,
    flattened: bool,
}

pub fn breath_morphology_probe(flow: Option<&DecodedChannel>) -> LabProbeResult {
    let Some(flow) = flow else {
        return gated(vec![
            "BRP Flow.40ms is required for breath morphology probing.".into(),
        ]);
    };

    if let Some(reason) = &flow.invalid_reason {
        return gated(vec![format!(
            "BRP Flow.40ms is marked unavailable: {reason}."
        )]);
    }

    if flow.sample_rate_hz <= 0.0 || !flow.sample_rate_hz.is_finite() {
        return gated(vec![
            "BRP Flow.40ms needs a finite sample rate before breath timing can be estimated."
                .into(),
        ]);
    }

    let values = flow
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return gated(vec![
            "BRP Flow.40ms contains no finite decoded physical samples.".into(),
        ]);
    }

    let breaths = segment_breaths(&values, flow.sample_rate_hz);
    if breaths.len() < MIN_COMPLETE_BREATHS {
        return gated(vec![format!(
            "Too few complete breaths were found for morphology probing: {} complete breath(s).",
            breaths.len()
        )]);
    }

    let breath_count = breaths.len();
    let median_duration = median(
        breaths
            .iter()
            .map(|breath| breath.duration_seconds)
            .collect(),
    );
    let balance = median(
        breaths
            .iter()
            .filter_map(|breath| {
                if breath.expiratory_seconds > 0.0 {
                    Some(breath.inspiratory_seconds / breath.expiratory_seconds)
                } else {
                    None
                }
            })
            .collect(),
    );
    let flattening_ratio =
        breaths.iter().filter(|breath| breath.flattened).count() as f64 / breath_count as f64;
    let unstable_ratio = unstable_breath_ratio(&breaths);

    let evidence = vec![
        format!("Breath count: {breath_count}"),
        format!(
            "Median breath duration: {:.2} s",
            median_duration.unwrap_or(0.0)
        ),
        format!(
            "Inspiratory/expiratory balance estimate: {:.2}",
            balance.unwrap_or(0.0)
        ),
        format!(
            "Flattening candidate ratio: {}",
            format_percent(flattening_ratio)
        ),
        format!("Unstable breath ratio: {}", format_percent(unstable_ratio)),
    ];

    let mut limitations = vec![
        "Breath morphology is a candidate signal for review, not a device-setting recommendation."
            .into(),
        "Segmentation is based on decoded flow sign changes and has not been validated against scored breaths."
            .into(),
    ];

    if breath_count < AVAILABLE_BREATHS {
        limitations.push(format!(
            "Short usable flow window: {breath_count} complete breaths limits confidence."
        ));
        return LabProbeResult::limited(
            PROBE_ID,
            PROBE_TITLE,
            "Exploratory breath morphology is limited by a short usable flow window.",
            evidence,
            limitations,
        );
    }

    LabProbeResult::available(
        PROBE_ID,
        PROBE_TITLE,
        "Exploratory breath morphology probe found enough decoded flow cycles for a candidate summary.",
        evidence,
    )
    .with_limitations(limitations)
}

fn gated(limitations: Vec<String>) -> LabProbeResult {
    LabProbeResult::gated(
        PROBE_ID,
        PROBE_TITLE,
        "Breath morphology is gated until BRP Flow.40ms contains enough decoded physical samples.",
        limitations,
    )
}

fn segment_breaths(values: &[f64], sample_rate_hz: f64) -> Vec<BreathSegment> {
    let threshold = flow_threshold(values);
    let signs = values
        .iter()
        .map(|value| sign_with_threshold(*value, threshold))
        .collect::<Vec<_>>();
    let starts = positive_run_starts(&signs);
    let mut breaths = Vec::new();

    for (start_index, start) in starts.iter().enumerate() {
        let end = starts.get(start_index + 1).copied().unwrap_or(values.len());
        if end <= *start {
            continue;
        }
        if let Some(segment) =
            describe_segment(&values[*start..end], &signs[*start..end], sample_rate_hz)
        {
            breaths.push(segment);
        }
    }

    breaths
}

fn flow_threshold(values: &[f64]) -> f64 {
    let mut magnitudes = values
        .iter()
        .map(|value| value.abs())
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if magnitudes.is_empty() {
        return 0.0;
    }
    magnitudes.sort_by(|left, right| left.total_cmp(right));
    let p95 = percentile_sorted(&magnitudes, 0.95);
    (p95 * 0.02).max(0.005)
}

fn sign_with_threshold(value: f64, threshold: f64) -> i8 {
    if value > threshold {
        1
    } else if value < -threshold {
        -1
    } else {
        0
    }
}

fn positive_run_starts(signs: &[i8]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut previous_non_positive = true;
    for (index, sign) in signs.iter().enumerate() {
        if *sign > 0 && previous_non_positive {
            starts.push(index);
        }
        previous_non_positive = *sign <= 0;
    }
    starts
}

fn describe_segment(values: &[f64], signs: &[i8], sample_rate_hz: f64) -> Option<BreathSegment> {
    let inspiratory_samples = signs.iter().filter(|sign| **sign > 0).count();
    let expiratory_samples = signs.iter().filter(|sign| **sign < 0).count();
    if inspiratory_samples == 0 || expiratory_samples == 0 {
        return None;
    }

    let duration_seconds = values.len() as f64 / sample_rate_hz;
    if !(0.8..=12.0).contains(&duration_seconds) {
        return None;
    }

    let inspiratory_values = values
        .iter()
        .zip(signs)
        .filter_map(|(value, sign)| (*sign > 0).then_some(*value))
        .collect::<Vec<_>>();
    let expiratory_values = values
        .iter()
        .zip(signs)
        .filter_map(|(value, sign)| (*sign < 0).then_some(value.abs()))
        .collect::<Vec<_>>();

    let inspiratory_peak = inspiratory_values.iter().copied().fold(0.0_f64, f64::max);
    let expiratory_peak = expiratory_values.iter().copied().fold(0.0_f64, f64::max);

    Some(BreathSegment {
        duration_seconds,
        inspiratory_seconds: inspiratory_samples as f64 / sample_rate_hz,
        expiratory_seconds: expiratory_samples as f64 / sample_rate_hz,
        inspiratory_peak,
        expiratory_peak,
        flattened: is_flattening_candidate(&inspiratory_values, inspiratory_peak),
    })
}

fn is_flattening_candidate(inspiratory_values: &[f64], inspiratory_peak: f64) -> bool {
    if inspiratory_values.len() < 6 || inspiratory_peak <= 0.0 {
        return false;
    }

    let near_peak = inspiratory_values
        .iter()
        .filter(|value| **value >= inspiratory_peak * 0.78)
        .count();
    let plateau_fraction = near_peak as f64 / inspiratory_values.len() as f64;
    let sorted = {
        let mut values = inspiratory_values.to_vec();
        values.sort_by(|left, right| left.total_cmp(right));
        values
    };
    let p75 = percentile_sorted(&sorted, 0.75);
    let p95 = percentile_sorted(&sorted, 0.95);
    let upper_band_narrow = (p95 - p75) <= inspiratory_peak * 0.12;

    plateau_fraction >= 0.30 && upper_band_narrow
}

fn unstable_breath_ratio(breaths: &[BreathSegment]) -> f64 {
    let amplitudes = breaths
        .iter()
        .map(|breath| breath.inspiratory_peak.max(breath.expiratory_peak))
        .collect::<Vec<_>>();
    let Some(median_amplitude) = median(amplitudes.clone()) else {
        return 0.0;
    };
    if median_amplitude <= 0.0 {
        return 0.0;
    }

    let unstable = amplitudes
        .iter()
        .filter(|amplitude| ((*amplitude - median_amplitude).abs() / median_amplitude) >= 0.40)
        .count();
    unstable as f64 / breaths.len() as f64
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.total_cmp(right));
    Some(percentile_sorted(&values, 0.50))
}

fn percentile_sorted(values: &[f64], quantile: f64) -> f64 {
    let index = ((values.len() - 1) as f64 * quantile).round() as usize;
    values[index]
}

fn format_percent(ratio: f64) -> String {
    format!("{:.0}%", ratio * 100.0)
}

trait LabProbeResultExt {
    fn with_limitations(self, limitations: Vec<String>) -> Self;
}

impl LabProbeResultExt for LabProbeResult {
    fn with_limitations(mut self, limitations: Vec<String>) -> Self {
        self.status = LabProbeStatus::Available;
        self.limitations = limitations;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::breath_morphology_probe;
    use crate::analysis::edf::DecodedChannel;
    use crate::analysis::lab_common::LabProbeStatus;

    #[test]
    fn gates_when_flow_channel_is_missing_or_invalid() {
        let missing = breath_morphology_probe(None);
        assert_eq!(missing.id, "breath_morphology");
        assert_eq!(missing.title, "Breath morphology");
        assert_eq!(missing.status, LabProbeStatus::Gated);
        assert!(missing.evidence.is_empty());
        assert!(missing
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Flow.40ms")));

        let mut invalid = flow_channel(vec![0.2, -0.2, 0.2, -0.2], 25.0);
        invalid.invalid_reason = Some("test invalid".into());
        let result = breath_morphology_probe(Some(&invalid));
        assert_eq!(result.status, LabProbeStatus::Gated);
        assert!(result
            .limitations
            .iter()
            .any(|limitation| limitation.contains("test invalid")));
    }

    #[test]
    fn gates_when_flow_is_empty_or_too_sparse_for_breath_segmentation() {
        let empty = breath_morphology_probe(Some(&flow_channel(vec![], 25.0)));
        assert_eq!(empty.status, LabProbeStatus::Gated);

        let too_sparse = breath_morphology_probe(Some(&flow_channel(
            vec![0.20, 0.15, -0.10, -0.15, 0.20, 0.15],
            25.0,
        )));
        assert_eq!(too_sparse.status, LabProbeStatus::Gated);
        assert!(too_sparse
            .limitations
            .iter()
            .any(|limitation| limitation.contains("Too few complete breaths")));
    }

    #[test]
    fn reports_breath_morphology_evidence_from_decoded_physical_flow_values() {
        let flow = flow_channel(
            [
                synthetic_breath(0.8, 1.0, false),
                synthetic_breath(0.9, 1.0, true),
                synthetic_breath(1.2, 0.8, true),
                synthetic_breath(0.7, 1.1, false),
                synthetic_breath(1.5, 1.0, true),
            ]
            .concat(),
            25.0,
        );

        let result = breath_morphology_probe(Some(&flow));

        assert_eq!(result.status, LabProbeStatus::Available);
        assert!(result.summary.contains("Exploratory"));
        assert!(result.evidence.iter().any(|line| line == "Breath count: 5"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Median breath duration: 4.00 s"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Inspiratory/expiratory balance estimate: 1.00"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Flattening candidate ratio: 60%"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line == "Unstable breath ratio: 20%"));
        assert!(result.limitations.iter().any(|line| {
            line.contains("candidate signal")
                && line.contains("not a device-setting recommendation")
        }));
    }

    #[test]
    fn returns_limited_when_only_a_short_usable_window_exists() {
        let flow = flow_channel(
            [
                synthetic_breath(0.8, 1.0, false),
                synthetic_breath(0.9, 1.0, false),
                synthetic_breath(1.0, 1.0, false),
            ]
            .concat(),
            25.0,
        );

        let result = breath_morphology_probe(Some(&flow));

        assert_eq!(result.status, LabProbeStatus::Limited);
        assert!(result.evidence.iter().any(|line| line == "Breath count: 3"));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("Short usable flow window")));
    }

    fn synthetic_breath(amplitude: f64, expiratory_scale: f64, flattened: bool) -> Vec<f64> {
        let mut values = Vec::new();
        if flattened {
            values.extend(expand_profile(
                &[0.18, 0.50, 0.78, 0.82, 0.80, 0.81, 0.79, 0.52, 0.22, 0.04],
                5,
            ));
        } else {
            values.extend(expand_profile(
                &[0.10, 0.30, 0.55, 0.80, 1.00, 0.82, 0.56, 0.28, 0.12, 0.03],
                5,
            ));
        }
        values.extend(expand_profile(
            &[
                -0.08, -0.30, -0.55, -0.75, -0.95, -0.80, -0.54, -0.25, -0.10, -0.03,
            ],
            5,
        ));
        values
            .into_iter()
            .map(|value| {
                if value >= 0.0 {
                    value * amplitude
                } else {
                    value * amplitude * expiratory_scale
                }
            })
            .collect()
    }

    fn flow_channel(values: Vec<f64>, sample_rate_hz: f64) -> DecodedChannel {
        DecodedChannel {
            label: "Flow.40ms".into(),
            unit: "L/s".into(),
            physical_min: -2.0,
            physical_max: 2.0,
            digital_min: -32768,
            digital_max: 32767,
            samples_per_record: sample_rate_hz.round() as usize,
            sample_rate_hz,
            values,
            invalid_reason: None,
        }
    }

    fn expand_profile(profile: &[f64], repeats: usize) -> Vec<f64> {
        profile
            .iter()
            .flat_map(|value| std::iter::repeat(*value).take(repeats))
            .collect()
    }
}
