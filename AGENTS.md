# Aerie / SuperCPAP Project Instructions

## Current Direction

Aerie is a local-only macOS desktop application for CPAP/PAP engineering analysis. Build it as a Tauri 2 app with a React + TypeScript frontend and a Rust backend. Do not rebuild the AI Studio/Cloud Run/PWA path unless the user explicitly asks to reopen that route.

## Privacy Boundary

- CPAP files must be processed locally on this Mac.
- Do not add Firebase, OAuth, Express upload APIs, Cloud Run, service workers, telemetry upload, analytics SDKs, or cloud explanation calls for v1.
- Do not copy the user's real EDF/CRC sample files into this repository.
- Tests may read the real sample files from their existing local iCloud path when present, but those tests must gracefully skip when the files are absent.
- Do not log raw EDF bytes, patient fields, serial numbers, machine identifiers, or filenames containing personal context unless the user explicitly requests a diagnostic dump.

## Product Boundary

- This is not medical advice and must not prescribe CPAP settings.
- User-facing language should say "supports a discussion", "compatible with", "insufficient signal", or "deserves review".
- Avoid "diagnosis", "prescription", "change your settings", or clinician-replacement framing.
- The UI should make data quality visible before any readout.
- Advanced engineering analysis belongs in Lab. Lab may be clever and exploratory, but every Lab feature must display its signal requirements, confidence limits, and validation status before showing conclusions.
- Lab features should be framed as hypotheses, probes, or candidate signals, not medical recommendations.

## Analysis Boundary

The analyzer must never invent metrics. In particular:

- Parse EDF headers and sample records, not only labels.
- Preserve physical min/max, digital min/max, samples per record, record duration, and channel units.
- Decode little-endian signed int16 EDF samples in EDF record order.
- Convert digital values to physical values before metrics.
- Mark missing or sentinel channels as unavailable instead of summarizing them.

Golden local sample checks:

- The sample directory is `/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914`.
- `20250914_211945_BRP.edf`, `20250914_211945_PLD.edf`, and `20250914_211945_SAD.edf` form one session despite a one-second header offset.
- PLD `Press.2s` is effectively constant around `15 cmH2O`.
- PLD `Leak.2s` max is about `0.340 L/s`; p95 is near `0.000 L/s`.
- SAD pulse and SpO2 in the sample decode as sentinel/invalid and must be reported unavailable, not as valid oxygenation.
- `20250914_205646_*` files are incomplete/header-only and must not produce usable fake metrics.

Primary local fixture folder:

- `/Users/ama/Desktop/Untitled Folder 2` is the working local test set when present. Do not copy it into the repo.
- It profiles as 159 EDF + 159 CRC files, with 37 complete BRP/PLD/SAD sessions grouped within a two-second start window.
- Best long fixture session: `20250816_233919_BRP.edf`, `20250816_233920_PLD.edf`, and `20250816_233920_SAD.edf`, about 7,320 seconds.
- This fixture is strong for flow, pressure, leak, session grouping, and invalid-oximetry gating. It is not a valid SpO2/oxygenation fixture because SAD oximetry decodes as sentinel-only.

## Build Pattern

- App root: `/Users/ama/SuperCPAP/app`.
- Use npm, Node 22, Rust/Cargo, and Tauri 2. Current pinned baseline: Rust `tauri` crate `2.10.3`, npm `@tauri-apps/api` and `@tauri-apps/cli` `2.10.1`.
- Keep one run entrypoint at `/Users/ama/SuperCPAP/app/script/build_and_run.sh`.
- Keep the Codex Run button config at `/Users/ama/SuperCPAP/.codex/environments/environment.toml`.
- Prefer tests before parser/metrics implementation.

## Reference Material

- Previous AI Studio prompt package remains useful only as product/context documentation under `/Users/ama/SuperCPAP/ai-studio-package`.
- The Claude standalone UI reference is `/Users/ama/Downloads/Aerie CPAP Suite (standalone).html`.
- The old Handy Tauri reference app is `/Users/ama/q2-edge-chat/handy_Pi`.
- The generated AI Studio repo reviewed at `/tmp/SuperCPAP-generated` is a cautionary scaffold, not a source of truth.
