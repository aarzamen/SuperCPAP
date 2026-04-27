use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabProbeStatus {
    Available,
    Limited,
    Gated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabProbeResult {
    pub id: String,
    pub title: String,
    pub status: LabProbeStatus,
    pub summary: String,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
}

impl LabProbeResult {
    pub fn available(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        evidence: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: LabProbeStatus::Available,
            summary: summary.into(),
            evidence,
            limitations: vec![
                "Exploratory engineering probe; not a device-setting recommendation.".into(),
            ],
        }
    }

    pub fn limited(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        evidence: Vec<String>,
        limitations: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: LabProbeStatus::Limited,
            summary: summary.into(),
            evidence,
            limitations,
        }
    }

    pub fn gated(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        limitations: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: LabProbeStatus::Gated,
            summary: summary.into(),
            evidence: Vec::new(),
            limitations,
        }
    }
}
