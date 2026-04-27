# Master Build Prompt For Google AI Studio

Build **Aerie**, a Google-hosted mobile-first PWA for CPAP/PAP data analysis.

Use the attached context documents as binding requirements:

- Product and safety brief.
- UI style guide.
- Analysis contract.
- EDF channel reference.

## Product Summary

Aerie analyzes uploaded CPAP/PAP export files, especially ResMed-style EDF/EDF+/CRC sets, and returns a clear, evidence-backed readout. The app helps answer whether the data supports a discussion about titration changes, while avoiding diagnosis, prescription, or medical advice.

This is a data-processing and project-development tool. It is not a medical device. It must warn users not to upload PII. Uploaded files may be processed server-side using Google's compute.

## Required Stack

Generate a complete full-stack React + TypeScript PWA suitable for Google AI Studio Build mode and later Cloud Run/GitHub export.

Required:

- React + TypeScript front end.
- Mobile-first PWA shell.
- Google OAuth / Firebase Auth gate before upload.
- Server-side upload and analysis route.
- No real API keys in client code.
- Use `@google/genai` only server-side for any Gemini explanation calls.
- Deterministic TypeScript parser/analysis modules.
- Structured JSON as the contract between analysis and UI.

## App Flow

1. **Sign in**
   - Google sign-in before upload.
   - Mobile-safe redirect flow if needed.

2. **Scope warning**
   - User must acknowledge:
     - This is a data-processing/project-development tool.
     - Do not upload PII.
     - Uploaded files may be processed server-side.
     - Outputs are not diagnosis, treatment, or prescription.

3. **Upload**
   - Accept multiple `.edf`, `.crc`, `.csv`, `.tsv` files.
   - Folder-style upload if supported by browser.
   - Show detected file types, count, size, likely date range, and source groups.
   - Let the user remove sources before analysis.

4. **Data quality**
   - Show parsed sessions, missing channels, corrupt files, unsupported files, and whether titration-relevant questions can be answered.

5. **Readout**
   - "What happened?"
   - "What might deserve attention?"
   - Key metrics: session duration, residual event burden if available, pressure, leak, flow limitation/snore if available, SpO2/pulse if available, trend.

6. **Evidence**
   - Numbered evidence rows with metric, value, source, strength, and caveat.

7. **Details**
   - Pressure trace/distribution.
   - Leak excursion timeline.
   - Flow/sampling availability.
   - SpO2/pulse summary if present.
   - Trend views when multiple sessions exist.

8. **Lab**
   - Visible but disabled in v1.
   - Show future labels: Breath Morphology, Cycling Behavior, Waveform Phenotypes, Inspiratory Limitation Research Mode.
   - State that v1 does not produce conclusions here.

## Deterministic Analysis Requirements

Implement an EDF parser that can:

- Read the EDF fixed header.
- Read signal labels, transducer, units, physical min/max, digital min/max, prefiltering, samples per record, and number of records.
- Convert digital int16 samples to physical values.
- Identify standard PAP channels from labels.
- Group related files by timestamp/session prefix where possible.
- Handle EDF annotations without crashing.
- Mark malformed, empty, or header-only files as limited/invalid.

Implement metrics:

- Session start time and duration.
- Channel availability and sample rates.
- Pressure median, P95, min, max, inter-percentile range, and time near upper bound if upper setting is available.
- Leak median, P95, max, and high-leak windows.
- Flow waveform availability and basic disturbance windows if enough flow data exists.
- Snore and flow-limitation summaries when channels exist.
- SpO2 and pulse summaries when channels exist; explicitly mark missing/invalid values.
- Data quality score.
- Conservative titration-support flags.

Implement finding categories:

- `data_quality`
- `pressure_support`
- `leak_support`
- `flow_limitation_support`
- `oxygenation_support`
- `insufficient_signal`
- `future_lab_unavailable`

Each finding must have:

- title
- summary
- severity: `info`, `attention`, or `limited`
- evidenceStrength: `strong`, `moderate`, `weak`, or `insufficient`
- metricRefs
- caveats
- safeTitrationLanguage

## Gemini Explanation Boundary

Gemini may only receive structured analysis JSON and produce:

- concise summary
- careful caveats
- user-facing interpretation
- report sections

Gemini must not:

- read raw uploaded files directly
- invent measurements
- diagnose
- prescribe settings
- say a change should be made
- override deterministic findings

The explanation prompt inside the app must explicitly say:

> You are explaining deterministic CPAP/PAP data-analysis outputs. Do not provide medical advice, diagnosis, or prescription. Use only the supplied JSON. If evidence is insufficient, say so.

## Visual Requirements

Preserve the feel of the Aerie reference UI:

- Wordmark: `aerie.` with the dot in calm clinical blue.
- Compact clinical-engineering instrument, not wellness app, not EMR.
- Background: off-white paper.
- Text: near-black ink.
- Supporting rules: cool grays.
- One signal color: calm blue.
- One warning color: sober amber.
- Thin-line charts.
- Hard axes.
- Threshold pins.
- Evidence ticks.
- Numbered rows.
- Sparse motion.
- Swipe-friendly mobile flow.

Do not use:

- Purple-blue AI gradients.
- Decorative blobs/orbs.
- Donut-chart theater.
- Motivational health copy.
- Nested cards.
- Desktop-only interactions.

## iOS PWA Requirements

Target iPhone 14 Pro Max / iOS Safari first.

Implement:

- `viewport-fit=cover`.
- `interactive-widget=resizes-content`.
- safe-area padding for fixed top/bottom UI.
- 44 px minimum touch targets.
- 16 px minimum input font size.
- PWA manifest.
- Apple mobile web app meta tags.
- 180 x 180 Apple touch icon.
- Service worker with safe update flow.
- iOS install hint: Share -> Add to Home Screen.

Use `100svh`, `100lvh`, or `100dvh` intentionally; do not rely on `100vh` for fixed full-screen mobile layouts.

## Security And Privacy Requirements

- Require sign-in before upload.
- Do not upload before explicit warning acknowledgement.
- Do not store uploaded files longer than needed unless the app clearly says so.
- Do not log raw file contents.
- Do not put secrets in browser code.
- Do not call Gemini directly from the browser with a privileged key.
- Show user-visible errors for failed auth, failed upload, unsupported files, corrupt EDF, missing channels, and failed analysis.

## Required File Tree

Generate a sensible full-stack project. Use this structure unless AI Studio requires a small adaptation:

```text
package.json
index.html
src/main.tsx
src/App.tsx
src/styles/tokens.css
src/styles/app.css
src/auth/firebaseAuth.ts
src/api/client.ts
src/types/analysis.ts
src/components/AppShell.tsx
src/components/ScopeWarning.tsx
src/components/UploadPanel.tsx
src/components/DataQualityPanel.tsx
src/components/Readout.tsx
src/components/EvidenceList.tsx
src/components/DetailView.tsx
src/components/LabFuture.tsx
src/components/charts/PressureChart.tsx
src/components/charts/LeakChart.tsx
src/components/charts/TrendSparkline.tsx
src/server/index.ts
src/server/routes/analyze.ts
src/server/routes/explain.ts
src/server/gemini/explainAnalysis.ts
src/analysis/edf/edfParser.ts
src/analysis/edf/channelMap.ts
src/analysis/sessionGrouping.ts
src/analysis/metrics.ts
src/analysis/findings.ts
src/analysis/sampleData.ts
src/analysis/verifyAnalysis.ts
public/manifest.webmanifest
public/sw.js
public/icons/apple-touch-icon.png
README.md
```

If AI Studio cannot emit binary PNG icons, generate SVG or CSS fallback icons and clearly mark the required PNG generation step in README. Do not block the rest of the app.

## Verification Requirements

Include a simple parser/analysis verification harness that can run against synthetic EDF-like buffers or bundled sample analysis objects. It must verify:

- EDF header parsing.
- Digital-to-physical scaling.
- Channel classification for Flow.40ms, Press.40ms, Leak.2s, MaskPress.2s, SpO2.1s, and Pulse.1s.
- Missing SpO2 handling.
- No unsafe titration language in generated deterministic findings.

Include exact commands in README for:

- install
- dev server
- verification
- build

## Final Output Constraints

Output a complete project with an explicit file tree.

Emit every generated file in full.

No fragments.

No placeholder comments.

No TODOs.

No omitted imports.

No fake libraries.

No markdown ellipses.

No "same as above".

Do not stop until every required file is emitted and verification commands are provided.

