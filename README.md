# Aerie CPAP Suite

Aerie is a local-only macOS desktop application for CPAP/PAP data exploration. It is built as a Tauri 2 app with a React/TypeScript frontend and a Rust analysis backend.

This repository is also a working exercise in user interface design, local data manipulation, binary file processing, and practical use of the Codex agentic coding suite to create, review, test, and iterate on an application. It started as an attempt to rescue useful ideas from a generated cloud app and rebuild them into a safer local desktop tool with real parsing and visible data-quality boundaries.

## Scope

Aerie is not a medical device, diagnostic tool, prescription engine, or replacement for a sleep clinician. The current app is framed as an engineering analysis and project-development exercise. It is intended to inspect exported PAP data, expose data quality, show which signals are usable, and support a better-informed discussion with qualified clinicians.

The app deliberately avoids cloud upload, Firebase, OAuth, remote APIs, telemetry, and analytics. Selected files are read locally on the Mac through Tauri commands. Real CPAP files and fixtures are not stored in this repository.

## What It Does Today

- Opens individual EDF/CRC files, whole folders, and multiple selected folders.
- Recursively scans selected sources while avoiding full-path display in the UI.
- Recognizes CPAP/PAP export roles such as BRP, PLD, SAD, and EVE.
- Groups complete BRP/PLD/SAD sessions within a close start-time window.
- Parses EDF headers and sample records rather than only reading labels.
- Preserves EDF signal metadata such as physical min/max, digital min/max, sample counts, record duration, channel labels, and units.
- Decodes little-endian signed int16 samples and scales them into physical units before computing summaries.
- Computes current scalar summaries for pressure, leak, flow, and oximetry availability.
- Rejects missing, incomplete, header-only, and sentinel-only signals rather than inventing metrics.
- Provides conservative findings with evidence and limitations.
- Includes an exploratory Lab section for breath morphology, trigger/cycle synchrony, leak-pressure interaction, oximetry coupling, instability windows, and counterfactual pressure/leak sandboxing.
- Displays a visual readout layer with source-quality bars, role completeness meters, session ribbons, range rails, status glyphs, evidence chips, Lab mini-symbols, and inline glossary affordances.

## Current Visual Language

The interface is intentionally dense but restrained. It uses faint engineering-style visual aids so the user can see signal availability, spread, and uncertainty before reading paragraphs of caveats.

Current reusable visual elements include:

- Source file quality stacked bars.
- BRP/PLD/SAD/EVE role completeness meters.
- SAD oximetry validity and sentinel gating strips.
- Session duration ribbons.
- Metric range rails with min, mean, median, p95, and max markers.
- Tone-aware finding cards for evidence, review, and limit states.
- Lab probe symbols and lightweight evidence meters.
- Inline glossary popovers for acronyms and technical terms.

## Architecture

```text
app/
  src/
    App.tsx              React UI and local interaction flow
    App.css              visual system and responsive layout
    visualModel.ts       testable visual-readout model helpers
  script/
    build_and_run.sh     one-command run/build/verify helper
    check_privacy_boundary.sh
    visualModel.test.ts
  src-tauri/
    src/
      analysis/          EDF parser, metrics, findings, source profiling, Lab probes
      commands.rs        Tauri command boundary
    Cargo.toml
```

Frontend state remains simple: the UI calls local Tauri commands and renders structured Rust results. Rust owns file IO, EDF parsing, metrics, session grouping, and privacy-sensitive paths.

## Build And Run

```bash
cd app
./script/build_and_run.sh
```

Build the macOS app:

```bash
cd app
./script/build_and_run.sh --build
```

The release app is built at:

```text
app/src-tauri/target/release/bundle/macos/Aerie.app
```

## Verification

Frontend privacy, visual-model, TypeScript, and Vite build checks:

```bash
cd app
npm run check
```

Rust parser, profiler, metrics, findings, Lab, session, and command tests:

```bash
cd app
cargo test --manifest-path src-tauri/Cargo.toml
```

Full desktop smoke check:

```bash
cd app
./script/build_and_run.sh --verify
```

Optional local fixture tests are ignored by default. They read local CPAP files when present and skip naturally when the files are absent.

```bash
cd app
cargo test --manifest-path src-tauri/Cargo.toml local -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml desktop_fixture -- --ignored --nocapture
```

## Known Shortcomings

- The app currently summarizes many signals with scalar statistics. It does not yet expose full decimated waveform previews to React.
- Several Lab visuals are string-derived evidence meters. They are useful for the current UI, but richer charts need typed structured payloads from Rust.
- Event-code mapping for trigger/cycle channels is still exploratory and needs validation against device documentation or known annotated data.
- Breath segmentation uses decoded flow sign changes and heuristic thresholds. It needs validation against scored breaths before it should be treated as more than an engineering probe.
- Oximetry is intentionally conservative. Sentinel-heavy SAD data is gated out, so current fixture coverage does not validate real oxygenation trend displays.
- Counterfactual bands are simple engineering hypothesis bands from observed leak and pressure behavior. They do not model sleep stage, mask fit, body position, comfort, clinical goals, or patient-reported symptoms.
- The app does not currently persist analysis sessions, export reports, compare nights, or build longitudinal trend views.
- There is no formal accessibility audit yet beyond basic keyboard/focus affordances and semantic labels.
- There is no packaging, signing, notarization, or installer workflow for distribution outside local development.
- The UI is optimized for a macOS desktop window. A separate iOS or mobile PWA path was intentionally set aside when the project moved local-only.

## Future Directions

- Add typed visual payloads from Rust:
  - decimated signal previews
  - event windows
  - confidence bands
  - breath segment summaries
  - oximetry drop and pulse-response annotations
- Add richer charts:
  - leak versus pressure scatter plots
  - flow waveform strips
  - breath morphology overlays
  - instability window timelines
  - oximetry and pulse paired traces
  - multi-night session calendars
- Add report export:
  - local PDF or HTML report
  - no raw patient identifiers by default
  - clear caveat and data-quality sections
- Add comparison workflows:
  - night-to-night trends
  - before/after device-setting context entered manually by the user
  - mask/leak artifact review
- Add stronger integrity checks:
  - CRC sidecar validation
  - duplicate-file detection
  - incomplete-session triage
- Add a structured glossary and visual legend drawer.
- Add UI tests or screenshot regression checks for common desktop window sizes.
- Add packaging and signing research for local macOS distribution.

## To-Do Queue

- Move Lab probe evidence from strings into typed structs.
- Add a `SignalVisual` or equivalent Rust-to-React contract for sampled previews and annotated windows.
- Add unit tests for each typed visual payload.
- Add chart components for waveform, scatter, band, and timeline displays.
- Add an analysis export format that keeps privacy boundaries intact.
- Expand fixture coverage with non-sentinel oximetry data.
- Add a settings/preferences panel for units, visual density, and advanced Lab visibility.
- Review UI copy for clinical-boundary language after each new feature.
- Add a short in-app visual legend explaining line styles, markers, and gating states.
- Document current EDF channel assumptions and any device-specific export-role uncertainty.

## Pending Engineering Questions

- Which device/export formats beyond the current ResMed-style EDF role naming should be supported?
- How should the app represent uncertainty when a channel is present but only partly physiologic?
- What is the minimum typed payload needed for waveform-level review without loading huge files into React?
- Should reports include raw file names, anonymized file names, or only session timestamps?
- How should manually entered context such as mask type, firmware mode, pressure settings, symptoms, or sleep position be handled without turning the app into medical advice?

## Development Notes

This app was developed with Codex as an agentic coding partner. The workflow intentionally used:

- code review of generated output before trusting it
- local-first architecture after cloud-upload concerns
- test-driven parser and metric work where practical
- parallel agents for independent analysis and implementation lanes
- explicit verification before completion claims
- conservative product language around medical interpretation

The project is therefore both a CPAP/PAP data-processing prototype and a record of how agentic development can be steered toward safer, more inspectable software.

## Privacy Reminder

Do not commit real CPAP exports, patient identifiers, serial numbers, raw EDF/CRC samples, or generated reports containing private context. The repository is for code, tests, documentation, and non-sensitive scaffolding only.
