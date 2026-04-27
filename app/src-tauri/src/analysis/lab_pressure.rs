use crate::analysis::{edf::DecodedChannel, lab_common::LabProbeResult};

#[cfg(test)]
use crate::analysis::lab_common::LabProbeStatus;

pub fn leak_pressure_probe(
    leak: Option<&DecodedChannel>,
    pressure: Option<&DecodedChannel>,
    mask_pressure: Option<&DecodedChannel>,
) -> LabProbeResult {
    let aligned = match aligned_required(leak, pressure) {
        Ok(aligned) => aligned,
        Err(limitations) => return gated_leak_pressure(limitations),
    };

    let leak_threshold = median(&aligned.leak);
    let mut baseline_pressure = Vec::new();
    let mut high_leak_pressure = Vec::new();
    for (leak_value, pressure_value) in aligned.leak.iter().zip(aligned.pressure.iter()) {
        if *leak_value > leak_threshold {
            high_leak_pressure.push(*pressure_value);
        } else {
            baseline_pressure.push(*pressure_value);
        }
    }

    if baseline_pressure.len() < 2 || high_leak_pressure.len() < 2 {
        return gated_leak_pressure(vec![
            "Leak samples did not separate into enough baseline and high-leak observations.".into(),
        ]);
    }

    let high_leak_fraction = high_leak_pressure.len() as f64 / aligned.len as f64;
    let correlation = pearson_correlation(&aligned.leak, &aligned.pressure);
    let high_leak_variability = standard_deviation(&high_leak_pressure);
    let baseline_variability = standard_deviation(&baseline_pressure);

    let mut evidence = vec![
        format!(
            "Compared {} aligned samples from decoded Leak.2s and Press.2s physical values.",
            aligned.len
        ),
        format!("Leak-pressure correlation: {correlation:.2}."),
        format!(
            "High-leak fraction above the session median leak ({leak_threshold:.3}): {:.1}%.",
            high_leak_fraction * 100.0
        ),
        format!(
            "Pressure variability during high leak: {high_leak_variability:.2} cmH2O versus baseline leak: {baseline_variability:.2} cmH2O."
        ),
    ];
    let mut limitations = aligned.limitations;

    if let Some(mask_pressure) = mask_pressure {
        match aligned_optional_mask(mask_pressure, aligned.len) {
            Ok(mask_values) => {
                let mean_delta = aligned
                    .pressure
                    .iter()
                    .zip(mask_values.iter())
                    .map(|(machine_pressure, mask_pressure)| machine_pressure - mask_pressure)
                    .sum::<f64>()
                    / aligned.len as f64;
                evidence.push(format!(
                    "Mask-pressure delta mean: {mean_delta:.2} cmH2O across aligned samples."
                ));
            }
            Err(reason) => limitations.push(reason),
        }
    } else {
        limitations.push(
            "MaskPress.2s was not available, so mask-pressure delta was not estimated.".into(),
        );
    }

    if limitations.is_empty() {
        LabProbeResult::available(
            "leak_pressure_interaction",
            "Leak-pressure interaction",
            "exploratory leak-pressure interaction probe is available from decoded PLD signals.",
            evidence,
        )
    } else {
        LabProbeResult::limited(
            "leak_pressure_interaction",
            "Leak-pressure interaction",
            "exploratory leak-pressure interaction probe is available with signal limitations.",
            evidence,
            limitations,
        )
    }
}

pub fn counterfactual_sandbox_probe(
    leak: Option<&DecodedChannel>,
    pressure: Option<&DecodedChannel>,
) -> LabProbeResult {
    let aligned = match aligned_required(leak, pressure) {
        Ok(aligned) => aligned,
        Err(limitations) => return LabProbeResult::gated(
            "counterfactual_sandbox",
            "Counterfactual sandbox",
            "Leak and pressure signals are required before counterfactual bands can be explored.",
            limitations,
        ),
    };

    let leak_threshold = median(&aligned.leak);
    let baseline_pressures = aligned
        .leak
        .iter()
        .zip(aligned.pressure.iter())
        .filter_map(|(leak_value, pressure_value)| {
            if *leak_value <= leak_threshold {
                Some(*pressure_value)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if baseline_pressures.len() < 2 {
        return LabProbeResult::gated(
            "counterfactual_sandbox",
            "Counterfactual sandbox",
            "The sandbox needs a baseline-leak pressure segment before hypothesis bands can be formed.",
            vec!["Too few baseline-leak samples remained after decoded signal alignment.".into()],
        );
    }

    let baseline_mean = mean(&baseline_pressures);
    let baseline_spread = standard_deviation(&baseline_pressures).max(0.05);
    let observed_pressure_spread = standard_deviation(&aligned.pressure).max(baseline_spread);
    let low_band = (
        baseline_mean - baseline_spread,
        baseline_mean + baseline_spread,
    );
    let high_band = (
        baseline_mean - observed_pressure_spread,
        baseline_mean + observed_pressure_spread,
    );

    let mut limitations = aligned.limitations;
    limitations.push(
        "Counterfactual bands are engineering hypotheses from observed signals only; they are not setting advice."
            .into(),
    );
    limitations.push(
        "Bands do not model sleep stage, mask fit, body position, or clinician goals.".into(),
    );

    LabProbeResult::limited(
        "counterfactual_sandbox",
        "Counterfactual sandbox",
        "Counterfactual sandbox produced engineering hypothesis bands from decoded leak and pressure signals.",
        vec![
            format!(
                "Low-confidence band around baseline-leak pressure behavior: {:.2}-{:.2} cmH2O.",
                low_band.0, low_band.1
            ),
            format!(
                "High-confidence band widened by observed pressure variability: {:.2}-{:.2} cmH2O.",
                high_band.0, high_band.1
            ),
            format!(
                "Bands used {} aligned samples and a median leak boundary of {leak_threshold:.3}.",
                aligned.len
            ),
        ],
        limitations,
    )
}

struct AlignedSignals {
    leak: Vec<f64>,
    pressure: Vec<f64>,
    len: usize,
    limitations: Vec<String>,
}

fn aligned_required(
    leak: Option<&DecodedChannel>,
    pressure: Option<&DecodedChannel>,
) -> Result<AlignedSignals, Vec<String>> {
    let leak = usable_channel(leak, "Leak.2s")?;
    let pressure = usable_channel(pressure, "Press.2s")?;
    let leak_values = finite_values(leak);
    let pressure_values = finite_values(pressure);
    if leak_values.is_empty() || pressure_values.is_empty() {
        return Err(vec![
            "Leak.2s and Press.2s must contain finite decoded physical samples.".into(),
        ]);
    }

    let min_len = leak_values.len().min(pressure_values.len());
    let max_len = leak_values.len().max(pressure_values.len());
    if min_len < 4 {
        return Err(vec![
            "Leak-pressure interaction needs at least four aligned samples.".into(),
        ]);
    }

    let difference = max_len - min_len;
    let allowed_difference = (max_len / 20).max(1);
    if difference > allowed_difference {
        return Err(vec![format!(
            "Leak.2s and Press.2s lengths differ too much for sensible truncation: {} versus {} samples.",
            leak_values.len(),
            pressure_values.len()
        )]);
    }

    let mut limitations = Vec::new();
    if difference > 0 {
        limitations.push(format!(
            "Signals were truncated to {min_len} aligned samples because channel lengths differed slightly."
        ));
    }

    Ok(AlignedSignals {
        leak: leak_values.into_iter().take(min_len).collect(),
        pressure: pressure_values.into_iter().take(min_len).collect(),
        len: min_len,
        limitations,
    })
}

fn aligned_optional_mask(mask_pressure: &DecodedChannel, len: usize) -> Result<Vec<f64>, String> {
    if let Some(reason) = &mask_pressure.invalid_reason {
        return Err(format!("MaskPress.2s was marked unavailable: {reason}."));
    }
    let values = finite_values(mask_pressure);
    if values.len() < len {
        return Err(format!(
            "MaskPress.2s had {} finite samples, fewer than the {len} leak-pressure samples.",
            values.len()
        ));
    }
    Ok(values.into_iter().take(len).collect())
}

fn usable_channel<'a>(
    channel: Option<&'a DecodedChannel>,
    required_label: &str,
) -> Result<&'a DecodedChannel, Vec<String>> {
    let Some(channel) = channel else {
        return Err(vec![format!(
            "{required_label} is required for this Lab probe."
        )]);
    };
    if let Some(reason) = &channel.invalid_reason {
        return Err(vec![format!("{required_label} is unavailable: {reason}.")]);
    }
    if channel.values.is_empty() {
        return Err(vec![format!(
            "{required_label} has no decoded physical samples."
        )]);
    }
    Ok(channel)
}

fn finite_values(channel: &DecodedChannel) -> Vec<f64> {
    channel
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect()
}

fn gated_leak_pressure(limitations: Vec<String>) -> LabProbeResult {
    LabProbeResult::gated(
        "leak_pressure_interaction",
        "Leak-pressure interaction",
        "Leak and pressure signals are required before this engineering probe can run.",
        limitations,
    )
}

fn pearson_correlation(left: &[f64], right: &[f64]) -> f64 {
    let left_mean = mean(left);
    let right_mean = mean(right);
    let mut numerator = 0.0;
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;

    for (left_value, right_value) in left.iter().zip(right.iter()) {
        let left_delta = left_value - left_mean;
        let right_delta = right_value - right_mean;
        numerator += left_delta * right_delta;
        left_sum += left_delta.powi(2);
        right_sum += right_delta.powi(2);
    }

    let denominator = left_sum.sqrt() * right_sum.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn standard_deviation(values: &[f64]) -> f64 {
    let value_mean = mean(values);
    let variance = values
        .iter()
        .map(|value| (value - value_mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    sorted[(sorted.len() - 1) / 2]
}

#[cfg(test)]
mod tests {
    use super::DecodedChannel;
    use super::{counterfactual_sandbox_probe, leak_pressure_probe, LabProbeStatus};

    #[test]
    fn leak_pressure_probe_reports_interaction_metrics_from_decoded_channels() {
        let leak = channel(
            "Leak.2s",
            "L/s",
            vec![0.00, 0.01, 0.02, 0.03, 0.20, 0.24, 0.28, 0.32],
        );
        let pressure = channel(
            "Press.2s",
            "cmH2O",
            vec![10.0, 10.1, 9.9, 10.0, 12.0, 13.0, 11.5, 14.0],
        );
        let mask_pressure = channel(
            "MaskPress.2s",
            "cmH2O",
            vec![9.9, 10.0, 9.8, 9.9, 11.2, 11.9, 10.8, 12.6],
        );

        let result = leak_pressure_probe(Some(&leak), Some(&pressure), Some(&mask_pressure));

        assert_eq!(result.id, "leak_pressure_interaction");
        assert_eq!(result.status, LabProbeStatus::Available);
        assert!(result.summary.contains("exploratory"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("correlation") && line.contains("0.92")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("High-leak fraction") && line.contains("50.0%")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("variability")
                && line.contains("0.96")
                && line.contains("0.07")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("Mask-pressure delta") && line.contains("0.55")));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("not a device-setting recommendation")));
    }

    #[test]
    fn leak_pressure_probe_gates_missing_invalid_empty_and_mismatched_inputs() {
        let leak = channel("Leak.2s", "L/s", vec![0.0, 0.1, 0.2, 0.3]);
        let pressure = channel("Press.2s", "cmH2O", vec![10.0, 11.0, 12.0, 13.0]);
        let empty = channel("Leak.2s", "L/s", vec![]);
        let short_pressure = channel("Press.2s", "cmH2O", vec![10.0]);
        let mut invalid = pressure.clone();
        invalid.invalid_reason = Some("sentinel values only".into());

        assert_eq!(
            leak_pressure_probe(None, Some(&pressure), None).status,
            LabProbeStatus::Gated
        );
        assert_eq!(
            leak_pressure_probe(Some(&empty), Some(&pressure), None).status,
            LabProbeStatus::Gated
        );
        assert_eq!(
            leak_pressure_probe(Some(&leak), Some(&invalid), None).status,
            LabProbeStatus::Gated
        );
        assert_eq!(
            leak_pressure_probe(Some(&leak), Some(&short_pressure), None).status,
            LabProbeStatus::Gated
        );
    }

    #[test]
    fn leak_pressure_probe_allows_small_length_difference_by_truncating() {
        let leak = channel("Leak.2s", "L/s", vec![0.0, 0.1, 0.2, 0.3, 0.4]);
        let pressure = channel("Press.2s", "cmH2O", vec![10.0, 10.2, 10.6, 11.0]);

        let result = leak_pressure_probe(Some(&leak), Some(&pressure), None);

        assert_ne!(result.status, LabProbeStatus::Gated);
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("Compared 4 aligned samples")));
        assert!(result
            .limitations
            .iter()
            .any(|line| line.contains("truncated")));
    }

    #[test]
    fn counterfactual_sandbox_outputs_hypothesis_bands_without_setting_advice() {
        let leak = channel(
            "Leak.2s",
            "L/s",
            vec![0.00, 0.01, 0.02, 0.03, 0.20, 0.24, 0.28, 0.32],
        );
        let pressure = channel(
            "Press.2s",
            "cmH2O",
            vec![10.0, 10.1, 9.9, 10.0, 12.0, 13.0, 11.5, 14.0],
        );

        let result = counterfactual_sandbox_probe(Some(&leak), Some(&pressure));

        assert_eq!(result.id, "counterfactual_sandbox");
        assert_eq!(result.status, LabProbeStatus::Limited);
        assert!(result.summary.contains("engineering hypothesis"));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("Low-confidence band")));
        assert!(result
            .evidence
            .iter()
            .any(|line| line.contains("High-confidence band")));

        let combined_text = format!(
            "{} {} {}",
            result.summary,
            result.evidence.join(" "),
            result.limitations.join(" ")
        )
        .to_ascii_lowercase();
        assert!(!combined_text.contains("change your settings"));
        assert!(!combined_text.contains("prescription"));
        assert!(!combined_text.contains("diagnosis"));
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
            sample_rate_hz: 0.5,
            values,
            invalid_reason: None,
        }
    }
}
