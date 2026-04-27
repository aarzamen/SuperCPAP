use crate::analysis::metrics::SessionMetrics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingTone {
    Evidence,
    Review,
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub tone: FindingTone,
    pub title: String,
    pub body: String,
    pub evidence: Vec<String>,
}

pub fn finding_for_session(metrics: &SessionMetrics) -> Vec<Finding> {
    let mut findings = Vec::new();

    if let Some(pressure) = &metrics.pressure {
        findings.push(Finding {
            tone: FindingTone::Evidence,
            title: "Pressure signal available".into(),
            body: format!(
                "Pressure ranged from {:.2} to {:.2} {} in this session; this supports discussion with the sleep clinician when paired with symptoms and device context.",
                pressure.min, pressure.max, pressure.unit
            ),
            evidence: vec![format!(
                "{}: median {:.2} {}, p95 {:.2} {}",
                pressure.channel, pressure.median, pressure.unit, pressure.p95, pressure.unit
            )],
        });
    }

    if let Some(leak) = &metrics.leak {
        let tone = if leak.p95 > 0.4 {
            FindingTone::Review
        } else {
            FindingTone::Evidence
        };
        findings.push(Finding {
            tone,
            title: "Leak signal measured".into(),
            body: format!(
                "Leak p95 was {:.2} {} with a maximum of {:.2} {}; the pattern deserves review as an engineering signal before interpreting pressure response.",
                leak.p95, leak.unit, leak.max, leak.unit
            ),
            evidence: vec![format!("{}: {} decoded samples", leak.channel, leak.samples)],
        });
    }

    if let Some(reason) = &metrics.oximetry.unavailable_reason {
        findings.push(Finding {
            tone: FindingTone::Limit,
            title: "Oximetry unavailable".into(),
            body: format!(
                "{reason}. Oxygenation cannot be summarized from this export, so any readout is limited to PAP engineering signals."
            ),
            evidence: Vec::new(),
        });
    }

    if findings.is_empty() {
        findings.push(Finding {
            tone: FindingTone::Limit,
            title: "Insufficient decoded signal".into(),
            body: "No supported session metrics were decoded, so the data is insufficient for a readout.".into(),
            evidence: Vec::new(),
        });
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::{finding_for_session, FindingTone};
    use crate::analysis::metrics::{OximetryMetrics, SessionMetrics, SignalStats};

    #[test]
    fn finding_language_is_conservative_and_evidence_bounded() {
        let metrics = SessionMetrics {
            pressure: Some(stats("Press.2s", "cmH2O", 10.0, 10.0, 10.0, 10.0, 10.0)),
            leak: Some(stats("Leak.2s", "L/sec", 0.0, 0.0, 0.0, 0.34, 0.01)),
            flow: None,
            oximetry: OximetryMetrics {
                spo2: None,
                pulse: None,
                unavailable_reason: Some(
                    "SAD oximetry channels contain sentinel values only".into(),
                ),
            },
        };

        let findings = finding_for_session(&metrics);
        let joined = findings
            .iter()
            .map(|finding| format!("{} {}", finding.title, finding.body))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();

        assert!(findings
            .iter()
            .any(|finding| finding.tone == FindingTone::Limit));
        assert!(joined.contains("supports discussion") || joined.contains("deserves review"));
        assert!(!joined.contains("diagnosis"));
        assert!(!joined.contains("prescribe"));
        assert!(!joined.contains("change your settings"));
    }

    fn stats(
        channel: &str,
        unit: &str,
        min: f64,
        median: f64,
        p95: f64,
        max: f64,
        mean: f64,
    ) -> SignalStats {
        SignalStats {
            channel: channel.into(),
            unit: unit.into(),
            samples: 10,
            min,
            median,
            p95,
            max,
            mean,
        }
    }
}
