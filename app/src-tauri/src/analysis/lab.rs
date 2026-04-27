use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabFeatureStatus {
    Queued,
    Near,
    Gated,
    Locked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabFeature {
    pub id: String,
    pub title: String,
    pub status: LabFeatureStatus,
    pub signal_requirements: Vec<String>,
    pub validation_posture: String,
    pub note: String,
    pub clinical_boundary: String,
}

pub fn lab_feature_catalog() -> Vec<LabFeature> {
    vec![
        LabFeature {
            id: "breath_morphology".into(),
            title: "Breath morphology".into(),
            status: LabFeatureStatus::Queued,
            signal_requirements: vec!["BRP Flow.40ms at 25 Hz".into()],
            validation_posture: "Requires breath segmentation against known flow samples before display.".into(),
            note: "Segment breaths, then score inspiratory flattening, recovery shape, and unstable clusters.".into(),
            clinical_boundary: "Exploratory morphology probe; does not prescribe device settings.".into(),
        },
        LabFeature {
            id: "trigger_cycle_synchrony".into(),
            title: "Trigger/cycle synchrony".into(),
            status: LabFeatureStatus::Queued,
            signal_requirements: vec!["BRP Flow.40ms".into(), "BRP TrigCycEvt.40ms".into()],
            validation_posture: "Requires event-code mapping and timing sanity checks before interpretation.".into(),
            note: "Look for timing mismatch patterns compatible with patient-machine asynchrony.".into(),
            clinical_boundary: "Asynchrony probe only; does not prescribe device settings.".into(),
        },
        LabFeature {
            id: "leak_pressure_interaction".into(),
            title: "Leak-pressure interaction".into(),
            status: LabFeatureStatus::Near,
            signal_requirements: vec!["PLD Leak.2s".into(), "PLD Press.2s".into(), "PLD MaskPress.2s".into()],
            validation_posture: "Near-term because decoded PLD channels already have golden sample checks.".into(),
            note: "Separate pressure-support interpretation from leak artifact before any discussion support.".into(),
            clinical_boundary: "Artifact and confidence probe; does not prescribe device settings.".into(),
        },
        LabFeature {
            id: "oximetry_coupling".into(),
            title: "Oximetry coupling".into(),
            status: LabFeatureStatus::Gated,
            signal_requirements: vec!["Valid SAD SpO2.1s".into(), "Valid SAD Pulse.1s".into()],
            validation_posture: "Gated because the current local sample has sentinel-only oximetry.".into(),
            note: "Only enabled when oximetry is physiologic; sentinel channels stay unavailable.".into(),
            clinical_boundary: "Oximetry association probe; does not prescribe device settings.".into(),
        },
        LabFeature {
            id: "instability_windows".into(),
            title: "Instability windows".into(),
            status: LabFeatureStatus::Queued,
            signal_requirements: vec![
                "BRP Flow.40ms".into(),
                "PLD RespRate.2s".into(),
                "PLD MinVent.2s".into(),
            ],
            validation_posture: "Requires windowing rules and minimum-duration thresholds before surfacing.".into(),
            note: "Flag clusters that deserve review without calling sleep stage or arousal state.".into(),
            clinical_boundary: "Instability signal probe; does not prescribe device settings.".into(),
        },
        LabFeature {
            id: "counterfactual_sandbox".into(),
            title: "Counterfactual sandbox".into(),
            status: LabFeatureStatus::Locked,
            signal_requirements: vec!["Validated pressure metrics".into(), "Validated leak metrics".into()],
            validation_posture: "Locked until deterministic metrics and findings are stable.".into(),
            note: "Explore what-if pressure and leak scenarios as engineering hypotheses, not setting advice.".into(),
            clinical_boundary: "Counterfactual sandbox only; does not prescribe device settings.".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{lab_feature_catalog, LabFeatureStatus};
    use std::collections::HashSet;

    #[test]
    fn catalog_includes_all_reviewed_advanced_lab_features() {
        let catalog = lab_feature_catalog();
        let titles: HashSet<_> = catalog
            .iter()
            .map(|feature| feature.title.as_str())
            .collect();

        assert!(titles.contains("Breath morphology"));
        assert!(titles.contains("Trigger/cycle synchrony"));
        assert!(titles.contains("Leak-pressure interaction"));
        assert!(titles.contains("Oximetry coupling"));
        assert!(titles.contains("Instability windows"));
        assert!(titles.contains("Counterfactual sandbox"));
    }

    #[test]
    fn every_lab_feature_declares_signal_requirements_and_validation_boundary() {
        let catalog = lab_feature_catalog();

        assert!(!catalog.is_empty());
        for feature in catalog {
            assert!(!feature.id.is_empty(), "feature id must be stable");
            assert!(
                !feature.signal_requirements.is_empty(),
                "{} needs explicit signal requirements",
                feature.title
            );
            assert!(
                !feature.validation_posture.is_empty(),
                "{} needs a validation posture",
                feature.title
            );
            assert!(
                feature
                    .clinical_boundary
                    .to_ascii_lowercase()
                    .contains("not prescribe"),
                "{} needs explicit non-prescriptive boundary",
                feature.title
            );
        }
    }

    #[test]
    fn feature_ids_are_unique_and_statuses_include_near_gated_and_locked() {
        let catalog = lab_feature_catalog();
        let ids: HashSet<_> = catalog.iter().map(|feature| feature.id.as_str()).collect();
        let statuses: HashSet<_> = catalog.iter().map(|feature| &feature.status).collect();

        assert_eq!(ids.len(), catalog.len());
        assert!(statuses.contains(&LabFeatureStatus::Near));
        assert!(statuses.contains(&LabFeatureStatus::Gated));
        assert!(statuses.contains(&LabFeatureStatus::Locked));
    }
}
