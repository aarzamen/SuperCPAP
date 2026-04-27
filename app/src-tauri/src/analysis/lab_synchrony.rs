use crate::analysis::edf::DecodedChannel;
use crate::analysis::lab_common::LabProbeResult;

const PROBE_ID: &str = "trigger_cycle_synchrony";
const PROBE_TITLE: &str = "Trigger/cycle synchrony";
const EVENT_EPSILON: f64 = 0.000_001;
const ALIGNMENT_WINDOW_SECONDS: f64 = 0.12;

pub fn trigger_cycle_synchrony_probe(
    flow: Option<&DecodedChannel>,
    trigger_cycle: Option<&DecodedChannel>,
) -> LabProbeResult {
    let flow = match flow {
        Some(channel) => channel,
        None => {
            return LabProbeResult::gated(
                PROBE_ID,
                PROBE_TITLE,
                "Trigger/cycle synchrony requires usable BRP flow before this exploratory probe can run.",
                vec!["BRP Flow.40ms is missing or unavailable.".into()],
            )
        }
    };

    if let Some(reason) = &flow.invalid_reason {
        return LabProbeResult::gated(
            PROBE_ID,
            PROBE_TITLE,
            "Trigger/cycle synchrony requires usable BRP flow before this exploratory probe can run.",
            vec![format!("BRP Flow.40ms is marked unavailable: {reason}.")],
        );
    }

    let finite_flow_samples = flow
        .values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .count();
    if flow.values.is_empty() {
        return LabProbeResult::gated(
            PROBE_ID,
            PROBE_TITLE,
            "Trigger/cycle synchrony requires usable BRP flow before this exploratory probe can run.",
            vec!["BRP Flow.40ms is empty.".into()],
        );
    }
    if finite_flow_samples < 2 {
        return LabProbeResult::gated(
            PROBE_ID,
            PROBE_TITLE,
            "Trigger/cycle synchrony requires at least two finite BRP flow samples.",
            vec!["BRP Flow.40ms does not contain enough finite decoded physical samples.".into()],
        );
    }

    let trigger_cycle = match trigger_cycle {
        Some(channel) => channel,
        None => {
            return LabProbeResult::limited(
                PROBE_ID,
                PROBE_TITLE,
                "BRP Flow.40ms is usable, but TrigCycEvt.40ms is absent, so this exploratory synchrony probe cannot compare event timing.",
                vec![format!(
                    "Flow signal contains {finite_flow_samples} finite decoded physical samples."
                )],
                vec!["TrigCycEvt.40ms channel is absent.".into()],
            )
        }
    };

    if let Some(reason) = &trigger_cycle.invalid_reason {
        return LabProbeResult::limited(
            PROBE_ID,
            PROBE_TITLE,
            "BRP Flow.40ms is usable, but trigger/cycle event samples are unavailable.",
            vec![format!(
                "Flow signal contains {finite_flow_samples} finite decoded physical samples."
            )],
            vec![format!("TrigCycEvt.40ms is marked unavailable: {reason}.")],
        );
    }

    let event_indices = nonzero_event_indices(trigger_cycle);
    if event_indices.is_empty() {
        return LabProbeResult::limited(
            PROBE_ID,
            PROBE_TITLE,
            "BRP Flow.40ms is usable, but no nonzero trigger/cycle event markers were present for this exploratory comparison.",
            vec![
                format!("Flow signal contains {finite_flow_samples} finite decoded physical samples."),
                format!(
                    "TrigCycEvt.40ms contains {} decoded physical samples with no nonzero event markers.",
                    trigger_cycle.values.len()
                ),
            ],
            vec![
                "Event code meanings are not decoded; this probe can only inspect nonzero marker timing."
                    .into(),
            ],
        );
    }

    let zero_crossings = flow_zero_crossings(&flow.values);
    let aligned_events =
        count_events_near_zero_crossings(&event_indices, trigger_cycle, flow, &zero_crossings);
    let event_rate = event_rate_per_minute(event_indices.len(), trigger_cycle, flow);
    let alignment_percent = aligned_events as f64 * 100.0 / event_indices.len() as f64;

    LabProbeResult::limited(
        PROBE_ID,
        PROBE_TITLE,
        "Trigger/cycle event markers can be compared with flow timing as an exploratory synchrony signal, but event-code meanings are not yet decoded.",
        vec![
            format!("Flow signal contains {finite_flow_samples} finite decoded physical samples."),
            format!(
                "{} nonzero trigger/cycle event samples were present.",
                event_indices.len()
            ),
            format!("{event_rate:.2} events/min across the decoded event channel."),
            format!(
                "{} of {} event samples were near flow zero crossings ({alignment_percent:.1}% rough alignment).",
                aligned_events,
                event_indices.len()
            ),
            format!(
                "{} flow zero crossings were visible in decoded physical flow samples.",
                zero_crossings.len()
            ),
        ],
        vec![
            "Event code meanings are not decoded; nonzero samples are treated only as generic trigger/cycle markers.".into(),
            "Alignment is a rough timing probe using decoded physical flow zero crossings, not a clinical interpretation.".into(),
        ],
    )
}

fn nonzero_event_indices(channel: &DecodedChannel) -> Vec<usize> {
    channel
        .values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if value.is_finite() && value.abs() > EVENT_EPSILON {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn flow_zero_crossings(values: &[f64]) -> Vec<usize> {
    values
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let previous = pair[0];
            let current = pair[1];
            if !previous.is_finite() || !current.is_finite() {
                return None;
            }

            let crosses_zero = (previous < 0.0 && current >= 0.0)
                || (previous > 0.0 && current <= 0.0)
                || previous == 0.0
                || current == 0.0;
            if crosses_zero {
                Some(index + 1)
            } else {
                None
            }
        })
        .collect()
}

fn count_events_near_zero_crossings(
    event_indices: &[usize],
    event_channel: &DecodedChannel,
    flow_channel: &DecodedChannel,
    zero_crossings: &[usize],
) -> usize {
    if zero_crossings.is_empty() || flow_channel.values.is_empty() {
        return 0;
    }

    let window_samples = alignment_window_samples(flow_channel.sample_rate_hz);
    event_indices
        .iter()
        .filter(|event_index| {
            let flow_index = event_index_to_flow_index(**event_index, event_channel, flow_channel);
            zero_crossings
                .iter()
                .any(|zero_crossing| zero_crossing.abs_diff(flow_index) <= window_samples)
        })
        .count()
}

fn event_index_to_flow_index(
    event_index: usize,
    event_channel: &DecodedChannel,
    flow_channel: &DecodedChannel,
) -> usize {
    if flow_channel.values.is_empty() {
        return 0;
    }

    let raw_index = if event_channel.sample_rate_hz > 0.0 && flow_channel.sample_rate_hz > 0.0 {
        let seconds = event_index as f64 / event_channel.sample_rate_hz;
        seconds * flow_channel.sample_rate_hz
    } else if !event_channel.values.is_empty() {
        event_index as f64 * flow_channel.values.len() as f64 / event_channel.values.len() as f64
    } else {
        0.0
    };

    raw_index
        .round()
        .clamp(0.0, flow_channel.values.len().saturating_sub(1) as f64) as usize
}

fn alignment_window_samples(flow_sample_rate_hz: f64) -> usize {
    if flow_sample_rate_hz > 0.0 {
        ((flow_sample_rate_hz * ALIGNMENT_WINDOW_SECONDS).round() as usize).max(1)
    } else {
        1
    }
}

fn event_rate_per_minute(
    event_count: usize,
    event_channel: &DecodedChannel,
    flow_channel: &DecodedChannel,
) -> f64 {
    let duration_seconds = if event_channel.sample_rate_hz > 0.0 {
        event_channel.values.len() as f64 / event_channel.sample_rate_hz
    } else if flow_channel.sample_rate_hz > 0.0 {
        flow_channel.values.len() as f64 / flow_channel.sample_rate_hz
    } else {
        0.0
    };

    if duration_seconds > 0.0 {
        event_count as f64 * 60.0 / duration_seconds
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::trigger_cycle_synchrony_probe;
    use crate::analysis::edf::DecodedChannel;
    use crate::analysis::lab_common::LabProbeStatus;

    #[test]
    fn gates_when_flow_is_missing_or_unusable() {
        let missing = trigger_cycle_synchrony_probe(None, None);
        assert_eq!(missing.id, "trigger_cycle_synchrony");
        assert_eq!(missing.title, "Trigger/cycle synchrony");
        assert_eq!(missing.status, LabProbeStatus::Gated);
        assert!(missing.summary.contains("requires usable BRP flow"));

        let mut invalid = channel("Flow.40ms", "L/s", vec![0.0, 1.0, -1.0], 25.0);
        invalid.invalid_reason = Some("sentinel-only channel".into());
        let result = trigger_cycle_synchrony_probe(Some(&invalid), None);
        assert_eq!(result.status, LabProbeStatus::Gated);
        assert!(result
            .limitations
            .iter()
            .any(|item| item.contains("sentinel-only")));

        let empty = channel("Flow.40ms", "L/s", vec![], 25.0);
        let result = trigger_cycle_synchrony_probe(Some(&empty), None);
        assert_eq!(result.status, LabProbeStatus::Gated);
        assert!(result.limitations.iter().any(|item| item.contains("empty")));
    }

    #[test]
    fn is_limited_when_trigger_cycle_channel_is_absent() {
        let flow = channel("Flow.40ms", "L/s", vec![-1.0, -0.4, 0.2, 0.9], 25.0);

        let result = trigger_cycle_synchrony_probe(Some(&flow), None);

        assert_eq!(result.status, LabProbeStatus::Limited);
        assert!(result.summary.contains("TrigCycEvt.40ms"));
        assert!(result
            .limitations
            .iter()
            .any(|item| item.contains("absent")));
    }

    #[test]
    fn counts_event_markers_and_reports_alignment_near_flow_zero_crossings() {
        let flow = channel(
            "Flow.40ms",
            "L/s",
            vec![-1.0, -0.5, 0.1, 0.8, 0.4, -0.2, -0.9, -0.4, 0.2, 0.7],
            25.0,
        );
        let events = channel(
            "TrigCycEvt.40ms",
            "",
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 3.0, 0.0],
            25.0,
        );

        let result = trigger_cycle_synchrony_probe(Some(&flow), Some(&events));

        assert_eq!(result.status, LabProbeStatus::Limited);
        assert!(result
            .evidence
            .iter()
            .any(|item| item.contains("3 nonzero trigger/cycle event samples")));
        assert!(result
            .evidence
            .iter()
            .any(|item| item.contains("450.00 events/min")));
        assert!(result
            .evidence
            .iter()
            .any(|item| item.contains("3 of 3 event samples were near flow zero crossings")));
        assert!(result
            .limitations
            .iter()
            .any(|item| item.contains("Event code meanings are not decoded")));
    }

    fn channel(label: &str, unit: &str, values: Vec<f64>, sample_rate_hz: f64) -> DecodedChannel {
        DecodedChannel {
            label: label.into(),
            unit: unit.into(),
            physical_min: 0.0,
            physical_max: 0.0,
            digital_min: 0,
            digital_max: 0,
            samples_per_record: values.len(),
            sample_rate_hz,
            values,
            invalid_reason: None,
        }
    }
}
