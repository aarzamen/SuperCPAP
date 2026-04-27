# Aerie CPAP Analysis Suite - Design Foundation

Date: 2026-04-27
Status: foundation draft
Reference UI: /Users/ama/Downloads/Aerie CPAP Suite (standalone).html

## Product Intent

Aerie is a mobile-first CPAP/PAP data analysis suite for technically literate users who want clear, evidence-backed signal analysis from PAP export files. It is a data-processing and project-development tool, not a medical device, diagnostic tool, prescription engine, or replacement for a sleep clinician.

The app should help answer:

- Is the uploaded data complete and clean enough to analyze?
- What happened during the session or date range?
- Does the data support a discussion about titration changes?
- Which signals support that interpretation, and where is the evidence weak?
- What deeper engineering views could exist later, without enabling unsupported v1 conclusions?

## Build Strategy For Gemini

The deliverable should be a master Google AI Studio prompt plus complete reference code modules for the hard parsing and analysis work.

Use two Gemini modes:

- Single-file prototype mode: for UI experiments, visual polish, gestures, chart interactions, and iPhone-safe layout. Output one raw HTML file only.
- Full-stack production mode: for the real app with OAuth, uploads, server-side processing, deterministic analysis modules, reports, and deployment.

For Gemini 3.1 Pro:

- Keep temperature at 1.0.
- Use high thinking for initial full-app generation and deep review.
- Set max output tokens to at least 32768.
- Put hard output constraints at the end of the prompt.
- Use a DEEP trigger for review/planning/debugging turns.
- Restart fresh after repeated failed repair loops.

Production prompt rule:

> Generate a complete full-stack project with an explicit file tree. No placeholders. No fragments. Every file listed must be emitted in full.

## Core Architecture

The production app should be a Google-hosted full-stack PWA with:

- OAuth-gated access to the front end.
- Upload flow for EDF, EDF+, CRC, and supported CSV/TSV summaries.
- Server-side analysis using Google's compute where useful.
- Deterministic parsing and statistics code as the source of truth.
- Gemini used only to summarize, organize, and explain structured outputs.
- Structured JSON exchanged between analysis engine, UI, and explanation layer.

Gemini must not invent measurements, diagnoses, prescriptions, or titration instructions. It may only explain evidence-tagged outputs produced by deterministic code.

## Safety And Scope

The app must present a clear warning before upload:

- This is an exercise in data processing and project development practice.
- Do not upload PII.
- Uploaded files may be processed server-side.
- Outputs are not diagnosis, treatment, or prescription.
- Titration language must use "supports considering", "compatible with", "insufficient signal", or "discuss with clinician", not command language.

The UI copy should avoid motivational wellness language and avoid fake certainty.

## V1 Analysis Scope

V1 stays inside well-supported PAP analysis concepts:

- File integrity and channel detection.
- Session duration and date range.
- Pressure summaries: median, 95th percentile, time near upper bound, pressure trend.
- Leak summaries: median, 95th percentile, high-leak periods, leak/event coincidence.
- Flow summaries where available: sampling rate, waveform availability, basic event-like disturbance windows.
- Residual event burden where reliable data exists.
- Snore and flow-limitation channels when present.
- SpO2 and pulse summaries when present, with explicit missing-data handling.
- Data-quality limitations.
- Conservative titration discussion flags.

V1 should not perform unsupported diagnosis, sleep staging as if it were PSG, firmware-specific clinical conclusions, or advanced breath-shape phenotyping.

## Evidence Layer

Every interpretation should carry:

- Metric inputs.
- Rule or heuristic used.
- Evidence strength.
- Caveats.
- Source category: device-derived, signal-derived, literature-derived, or insufficient signal.

Initial evidence anchors:

- AASM PAP titration concepts: reduce obstructive respiratory events, snoring, and flow-limitation/RERA-like signals while maintaining acceptable leak and oxygenation.
- EDF/EDF+ file format specs for parsing.
- Device-export-specific observed channel maps such as Flow.40ms, Press.40ms, MaskPress.2s, Leak.2s, RespRate.2s, TidVol.2s, MinVent.2s, Snore.2s, FlowLim.2s, Pulse.1s, and SpO2.1s.

## UI Reference To Preserve

The Claude-designed Aerie standalone prototype is the visual reference. Preserve these qualities:

- Brand: "aerie." with a tiny clinical-blue dot.
- Tone: compact clinical-engineering instrument, not wellness dashboard and not EMR.
- Palette: off-white paper, near-black ink, cool gray rules, one calm signal blue, one sober amber warning.
- Typography: Inter for readable UI, JetBrains Mono for instrument labels and values, optional restrained serif accent only if needed.
- Visual language: thin lines, hard axes, faint engineering-paper grid, no gradient theater, no candy charts.
- Motion: subtle breathing/cloud intro, small swipe hints, quiet page transitions.
- Layout: iPhone-first, one-handed, swipeable flow, data-rich but scannable.
- Charts: pressure traces, leak co-trace, strip plots, histograms, trend sparks, threshold pins, evidence ticks.

The prototype's "Lab" tab behavior is correct for v1: visible as a future-facing affordance, disabled or low-emphasis, clearly marked as future research mode.

## Screen Flow

1. Sign in
   - Google OAuth.
   - Minimal gate before any upload.

2. Scope warning
   - Must acknowledge no PII, server-side processing, and not-medical-advice scope.

3. Upload
   - Accept folder or files.
   - Show detected file count, file types, likely device/export family, date range, and integrity score.
   - Support adding multiple sources and deduping sessions.

4. Data quality
   - Parsed channels.
   - Missing channels.
   - Corrupt records.
   - Unsupported files.
   - Whether the dataset can answer titration-relevant questions.

5. Night or range readout
   - "What happened?"
   - "What might deserve attention?"
   - Key metrics: residual event burden, leak, pressure, SpO2 if present, session duration, trend.

6. Evidence
   - Rows of supporting facts and caveats.
   - Human-readable but grounded in structured JSON.

7. Detail views
   - Pressure distribution and traces.
   - Leak excursions and coincidence windows.
   - Flow-limitation/snore context.
   - Trend views across nights.

8. Lab
   - Disabled in v1.
   - Hints at breath morphology, cycling behavior, waveform phenotypes, and inspiratory limitation research mode.

## Analysis Engine Boundaries

Deterministic code owns:

- EDF header parsing.
- Signal scaling from digital to physical values.
- Channel classification.
- Session grouping.
- CRC/integrity checks where feasible.
- Sampling-rate calculation.
- Pressure/leak/event summaries.
- Titration-support flags.
- Evidence JSON.

Gemini owns:

- Plain-language summary.
- UI copy for structured findings.
- Report organization.
- Explanation of caveats.
- User-facing phrasing bounded by rule outputs.

Gemini must not:

- Recompute metrics from raw uploaded data in prose.
- Infer diagnosis.
- Prescribe settings.
- Override deterministic flags.
- Present future Lab metrics as active v1 conclusions.

## Design Notes From The Prototype

Keep:

- Splash: calibration/breath motif and restrained wordmark.
- Disclosures: checkbox-style acknowledgements with sober language.
- Upload: folder-first ingest, detected sources, integrity bar, deduped nights.
- Readout: compact headline card, attention items, drill targets.
- Evidence: simple numbered rows with values and caveats.
- Details: chart first, then evidence rows.
- Future Lab: visible but disabled.

Change for production:

- Replace "No PII leaves this device" with accurate server-side upload language.
- Replace demo constants with real structured analysis JSON.
- Remove in-browser Babel/dev React for production.
- Add true PWA metadata, service worker, safe-area handling, and iOS install behavior.
- Add OAuth gate and upload lifecycle states.

## Open Questions

- Whether uploads should be retained temporarily, deleted immediately after analysis, or retained per signed-in user.
- Whether the first production target is AI Studio full-stack React or a single generated project exported to GitHub.
- Whether reports should be downloadable as PDF, JSON, or both.
- Whether the first parser implementation should prioritize ResMed-style EDF/CRC only or include OSCAR CSV/TSV immediately.

