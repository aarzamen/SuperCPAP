# Aerie Product And Safety Brief

## Product

Aerie is a CPAP/PAP data analysis suite for technically literate users. It accepts CPAP/PAP export files and produces a clear, evidence-backed engineering readout.

It is intentionally not a wellness coach, not a physician substitute, and not a sleep-lab replacement.

## Required Warning Before Upload

The app must require acknowledgement of these points before files can be uploaded:

1. Aerie is a data-processing and project-development tool.
2. Do not upload personally identifiable information.
3. Uploaded files may be processed server-side using Google-hosted compute.
4. Outputs are not diagnosis, treatment, prescription, or medical advice.
5. Any titration-related output is only a basis for discussion with a clinician.

## Acceptable Language

Use:

- "supports considering"
- "compatible with"
- "insufficient signal"
- "limited by missing channel"
- "discuss with clinician"
- "not enough evidence from uploaded data"
- "artifact likely"
- "data quality limits this conclusion"

Avoid:

- "diagnosis"
- "you have"
- "prescribe"
- "change your setting"
- "set pressure to"
- "treat"
- "cure"
- "safe to ignore"

## Titration Framing

The app may answer:

- Does the data support a titration discussion?
- Which signals contribute to that support?
- Which signals argue against changing settings?
- Which missing signals limit confidence?

The app must not answer:

- What exact pressure should this person use?
- Whether the user has a specific disorder.
- Whether a clinician is wrong.
- Whether the user should bypass clinical care.

## Evidence Strength

Every finding must use one of:

- `strong`: multiple relevant signals agree and data quality is good.
- `moderate`: relevant signal exists but has caveats or limited sample size.
- `weak`: pattern exists but could be artifact or missing corroboration.
- `insufficient`: cannot support the interpretation from uploaded data.

## Privacy

The UI must say that users should not upload PII. The app should avoid displaying or storing patient identifiers from EDF headers. If patient/header metadata exists, ignore or redact it in UI and logs.

Server logs must not include raw file contents or patient fields.

