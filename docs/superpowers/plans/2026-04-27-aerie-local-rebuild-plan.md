# Aerie Local Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild Aerie primarily from this coding workspace into a trustworthy CPAP/PAP analysis PWA with real deterministic EDF parsing, real auth boundaries, and the Claude-designed mobile UI direction.

**Architecture:** Treat the Gemini-generated repo as a disposable scaffold, not the source of truth. Keep the Aerie UI/UX direction and selected React shell pieces, but replace the auth, API enforcement, parser, metrics, verification, and PWA layers. Build in vertical slices: first local deterministic analysis that passes against the real EDF sample files, then wire upload/API, then restore mobile UI polish, then deploy.

**Tech Stack:** React + TypeScript + Vite, Node/Express server or Cloud Run-compatible server entry, Firebase Auth / Google OAuth, deterministic TypeScript EDF parser, Vitest or Node test harness, PWA manifest/service worker, optional Gemini explanation layer after metrics are trusted.

---

## Current Assessment

The generated repo at `/tmp/SuperCPAP-generated` is useful as a scaffold but not trustworthy as an analyzer.

Keep as reference:

- Visual direction from `/Users/ama/Downloads/Aerie CPAP Suite (standalone).html`.
- Prompt/package docs in `/Users/ama/SuperCPAP/ai-studio-package`.
- High-level screen sequence from generated React app.
- Basic Express/Vite shape if we want to avoid re-scaffolding.
- Type contracts in `src/types/analysis.ts`, after tightening.

Rewrite:

- `src/auth/firebaseAuth.ts`
- `src/server/routes/analyze.ts`
- `src/server/routes/explain.ts`
- `src/server/gemini/explainAnalysis.ts`
- `src/analysis/edf/*`
- `src/analysis/metrics.ts`
- `src/analysis/findings.ts`
- `src/analysis/sessionGrouping.ts`
- `src/analysis/verifyAnalysis.ts`
- `public/sw.js`

Delete or replace:

- Mock auth.
- Hardcoded metrics.
- Placeholder charts.
- Placeholder service worker.
- Incomplete verification harness.
- Any UI copy naming the app `SuperCPAP` instead of `Aerie`.

## Target Repository Course

Use `/Users/ama/SuperCPAP` as the working project root.

First action during implementation:

```bash
cd /Users/ama/SuperCPAP
git clone https://github.com/aarzamen/SuperCPAP.git app
```

If `/Users/ama/SuperCPAP/app` already exists, update it instead:

```bash
cd /Users/ama/SuperCPAP/app
git status --short --branch
git pull --ff-only
```

Do not continue building in `/tmp/SuperCPAP-generated`; it is a review clone.

## Execution Strategy

Use subagent-driven development with disjoint lanes. Do not run all lanes against the same files at once.

Recommended delegation:

1. **Parser Worker**
   - Owns `src/analysis/edf/*`, `src/analysis/sessionGrouping.ts`, parser tests.
   - Does not touch UI or auth.

2. **Metrics Worker**
   - Owns `src/analysis/metrics.ts`, `src/analysis/findings.ts`, analysis contract tests.
   - Depends on parser output contract.

3. **Auth/API Worker**
   - Owns `src/auth/*`, `src/server/auth/*`, `src/server/routes/*`, `server.ts`.
   - Does not touch parser internals.

4. **UI Worker**
   - Owns `src/components/*`, `src/styles/*`, `src/App.tsx`, `src/main.tsx`.
   - Consumes stable `AnalysisResult`.

5. **PWA/Deploy Worker**
   - Owns `public/*`, `index.html`, `README.md`, deployment docs.
   - Runs after auth/API and UI settle.

Each worker must run its own verification and report exact files changed.

## Milestone 0: Repo Stabilization

**Files:**
- Create or modify: `/Users/ama/SuperCPAP/app`
- Modify: `/Users/ama/SuperCPAP/app/package.json`
- Modify: `/Users/ama/SuperCPAP/app/tsconfig.json`

- [ ] Clone the GitHub repo into `/Users/ama/SuperCPAP/app`.
- [ ] Run `npm ci`.
- [ ] Run `npm run lint` and capture current failures.
- [ ] Rename package metadata from `react-example` to `aerie-cpap-suite`.
- [ ] Add `vitest` if we choose a real test runner:

```bash
cd /Users/ama/SuperCPAP/app
npm install -D vitest
```

- [ ] Add scripts:

```json
{
  "test": "vitest run",
  "verify:analysis": "tsx src/analysis/verifyAnalysis.ts",
  "check": "npm run lint && npm run test && npm run build"
}
```

- [ ] Fix the existing TypeScript import errors before any feature work:
  - `src/analysis/sessionGrouping.ts` import should be `../types/analysis`, not `../../types/analysis`.
  - `src/components/UploadPanel.tsx` should import React types or avoid `React.ChangeEvent`.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run lint
```

Expected: no TypeScript errors after this milestone.

## Milestone 1: Deterministic EDF Parser

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/src/analysis/edf/headerParser.ts`
- Rewrite: `/Users/ama/SuperCPAP/app/src/analysis/edf/edfParser.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/analysis/edf/channelMap.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/types/analysis.ts`
- Create: `/Users/ama/SuperCPAP/app/src/analysis/edf/edfParser.test.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/analysis/verifyAnalysis.ts`

Parser responsibilities:

- Parse EDF fixed header by byte offset.
- Parse per-signal arrays in EDF order.
- Preserve digital min/max and physical min/max.
- Compute sample rate from samples-per-record and record duration.
- Decode little-endian int16 samples from data records.
- Convert samples to physical values.
- Support EDF+D annotations without crashing.
- Treat record count `-1`, zero data, or header-only files as limited, not fatal.
- Redact patient/header identity fields from UI and logs.

Key output shape:

```ts
export interface DecodedChannel extends ChannelSummary {
  digitalMin: number;
  digitalMax: number;
  samplesPerRecord: number;
  values: Float64Array;
}

export interface ParsedEdfPayload {
  fileName: string;
  role: "brp" | "pld" | "sad" | "eve" | "crc" | "csv" | "unknown";
  format: "edf" | "edf_plus" | "crc" | "csv" | "tsv" | "unsupported";
  valid: boolean;
  headerBytes?: number;
  recordCount?: number;
  recordDurationSeconds?: number;
  startTime?: string;
  channels: DecodedChannel[];
  warnings: string[];
}
```

Real sample verification files:

```text
/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914/20250914_211945_BRP.edf
/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914/20250914_211945_PLD.edf
/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914/20250914_211945_SAD.edf
```

Expected sample facts:

- BRP has `Flow.40ms`, `Press.40ms`, `TrigCycEvt.40ms`, `Crc16`.
- BRP has 16 records, 60 seconds per record, 25 Hz flow and pressure.
- PLD has `MaskPress.2s`, `Press.2s`, `Leak.2s`, `RespRate.2s`, `TidVol.2s`, `MinVent.2s`, `Snore.2s`, `FlowLim.2s`.
- PLD has 16 records, 60 seconds per record, 0.5 Hz metric channels.
- SAD has `Pulse.1s` and `SpO2.1s`, but values in this sample should be treated as invalid/missing.

Golden checks from the known sample set:

- Group `20250914_211945_BRP`, `20250914_211945_PLD`, and `20250914_211945_SAD` into one session despite the one-second header offset.
- Report `20250914_205646_*` files as header-only or incomplete, not as usable sessions with invented metrics.
- PLD `Press.2s` is effectively constant at `15 cmH2O`.
- PLD `Leak.2s` has p95 near `0.000 L/s` and max about `0.340 L/s`.
- SAD pulse and SpO2 decode as sentinel/invalid values in this sample and must produce an unavailable reason, not median oxygenation.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run verify:analysis
npm run test -- src/analysis/edf/edfParser.test.ts
```

Expected:

- Parser reports correct labels and sample counts.
- Digital-to-physical scaling produces plausible values.
- Invalid SAD values are detected as unavailable.

## Milestone 2: Session Grouping And Metrics

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/src/analysis/sessionGrouping.ts`
- Rewrite: `/Users/ama/SuperCPAP/app/src/analysis/metrics.ts`
- Rewrite: `/Users/ama/SuperCPAP/app/src/analysis/findings.ts`
- Create: `/Users/ama/SuperCPAP/app/src/analysis/metrics.test.ts`
- Create: `/Users/ama/SuperCPAP/app/src/analysis/findings.test.ts`

Session grouping:

- Group by filename timestamp prefix when present.
- Reconcile BRP/PLD/SAD start times that differ by one second.
- Keep multiple sessions separate.
- Attach CRC sidecars to the matching EDF role.

Metrics:

- Pressure: min, median, p95, max from actual pressure values.
- Mask pressure: min, median, p95, max from `MaskPress.2s`.
- Leak: min, median, p95, max, high-leak windows from actual `Leak.2s`.
- Flow: sample rate and duration from actual `Flow.40ms`.
- Snore: summary from `Snore.2s`.
- Flow limitation: summary from `FlowLim.2s`.
- SpO2/pulse: unavailable when values are sentinel/missing.
- Data quality: good/limited/poor/unusable from parsed record counts, missing channels, and invalid channels.

Finding logic:

- Never produce a setting change.
- Use conservative categories:
  - `data_quality`
  - `pressure_support`
  - `leak_support`
  - `flow_limitation_support`
  - `oxygenation_support`
  - `insufficient_signal`
  - `future_lab_unavailable`
- If leak is high, pressure interpretation confidence drops.
- If SpO2 invalid/missing, oxygenation support is `insufficient`.
- If duration is short, titration support cannot be `strong`.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run verify:analysis
npm run test -- src/analysis/metrics.test.ts src/analysis/findings.test.ts
```

Expected for the real sample:

- No fake SpO2 values.
- Pressure/leak values match decoded sample-derived ranges.
- Titration support is conservative because the sample is only 16 minutes.

## Milestone 3: API And Auth Boundary

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/src/auth/firebaseAuth.ts`
- Create: `/Users/ama/SuperCPAP/app/src/server/auth/verifyFirebaseToken.ts`
- Modify: `/Users/ama/SuperCPAP/app/server.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/server/routes/analyze.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/server/routes/explain.ts`
- Create: `/Users/ama/SuperCPAP/app/src/server/routes/health.ts`

Auth requirements:

- Front end uses Firebase Auth / Google sign-in.
- Server routes require a valid ID token for `/api/analyze` and `/api/explain`.
- Client sends `Authorization: Bearer <idToken>`.
- Unauthenticated API calls return `401`, before parsing files or calling Gemini.
- Server logs never include raw EDF bytes or patient fields.

Server upload requirements:

- Memory upload is acceptable for v1 with explicit size limits.
- Limit accepted extensions to `.edf`, `.crc`, `.csv`, `.tsv`.
- Reject empty upload before analysis.
- Return structured error JSON.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run dev
curl -i -X POST http://localhost:3000/api/analyze
```

Expected:

```text
HTTP/1.1 401 Unauthorized
```

Then test an authenticated request through the UI or with a captured Firebase ID token.

## Milestone 4: UI Rebuild From Aerie Reference

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/src/App.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/AppShell.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/ScopeWarning.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/UploadPanel.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/DataQualityPanel.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/Readout.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/EvidenceList.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/DetailView.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/components/LabFuture.tsx`
- Rewrite: `/Users/ama/SuperCPAP/app/src/styles/tokens.css`
- Rewrite: `/Users/ama/SuperCPAP/app/src/styles/app.css`

UI requirements:

- Brand is `aerie.`, not `SuperCPAP`.
- Preserve the Claude standalone feel: off-white paper, near-black ink, signal blue, sober amber, monospace instrument labels.
- Mobile-first for iPhone 14 Pro Max.
- Sign-in screen first.
- Scope warning before upload.
- Upload screen with source summary.
- Data quality screen before readout if analysis is limited.
- Readout screen with "what happened" and "what might deserve attention".
- Evidence screen with numbered rows.
- Detail views with chart first, evidence rows second.
- Lab visible but disabled.

Charts:

- Replace placeholder chart divs with simple SVG charts driven by `AnalysisResult`.
- Pressure chart consumes decoded pressure summaries and optional series downsample.
- Leak chart consumes leak windows and summary.
- Trend chart is disabled until multi-session data exists.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run lint
npm run build
```

Manual checks:

- On 430 x 932 viewport, no overlap.
- Touch targets are at least 44 px.
- No local-only upload disclosure remains.
- No "diagnosis", "prescription", or "change your setting" language in user-facing copy except inside explicit forbidden-language lists in tests/docs.

## Milestone 5: PWA And Deployment

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/public/sw.js`
- Modify: `/Users/ama/SuperCPAP/app/public/manifest.webmanifest`
- Modify: `/Users/ama/SuperCPAP/app/index.html`
- Modify: `/Users/ama/SuperCPAP/app/README.md`
- Create: `/Users/ama/SuperCPAP/app/public/icons/aerie-icon.svg`

PWA requirements:

- Use `viewport-fit=cover` and `interactive-widget=resizes-content`.
- Do not set `user-scalable=0`.
- Apple mobile web app tags.
- Proper manifest with name, id, start_url, scope, display, theme color.
- Service worker:
  - network-first for HTML navigations
  - cache-first for static built assets
  - no caching for `/api/analyze`, `/api/explain`, uploads, or uploaded content
  - no blind `skipWaiting()` in install
  - user-gated update path if implemented

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run build
npm run start
```

Manual checks:

- App loads from production server.
- Service worker does not cache API responses.
- Install metadata is present.

## Milestone 6: Gemini Explanation Layer

**Files:**
- Rewrite: `/Users/ama/SuperCPAP/app/src/server/gemini/explainAnalysis.ts`
- Modify: `/Users/ama/SuperCPAP/app/src/server/routes/explain.ts`
- Create: `/Users/ama/SuperCPAP/app/src/server/gemini/explainAnalysis.test.ts`

Only implement after deterministic analysis is trusted.

Requirements:

- Use the current intended model from environment, not hardcoded `gemini-2.5-pro`.
- Server-only.
- Auth required.
- Input is `AnalysisResult`, not raw files.
- Output is optional explanatory markdown.
- Explanation must not include forbidden phrases.
- If Gemini fails, UI still displays deterministic readout.

Verification:

```bash
cd /Users/ama/SuperCPAP/app
npm run test -- src/server/gemini/explainAnalysis.test.ts
```

Expected:

- Prompt includes safety boundary.
- Forbidden claims are checked.
- Failure path returns a safe fallback, not a crashed readout.

## Milestone 7: Final Verification And Deployment

Run:

```bash
cd /Users/ama/SuperCPAP/app
npm run check
npm run verify:analysis
```

Then verify deployed endpoint behavior:

```bash
curl -i -X POST https://supercpap-63587206447.us-west1.run.app/api/analyze
```

Expected:

```text
401 Unauthorized
```

Do not upload real CPAP files to a public deployment from automation unless the user explicitly approves that specific upload after acknowledging the data-sensitivity risk.

## Risk Register

- **Biggest product risk:** fake or weak analysis presented with polished UI.
  - Mitigation: parser/metrics before UI polish.

- **Biggest security risk:** front-end-only auth.
  - Mitigation: server-side Firebase ID token verification on every sensitive route.

- **Biggest technical risk:** EDF parsing errors silently producing plausible numbers.
  - Mitigation: real sample-based tests with known expected values.

- **Biggest UX risk:** the app feels finished before it is honest.
  - Mitigation: visible data quality and limitations before readout.

## Stop Conditions

Pause implementation and ask for direction if:

- Firebase project credentials are unavailable.
- Cloud Run deployment requires account/manual setup.
- Real EDF test fixtures cannot be used locally.
- The generated repo structure fights the rebuild harder than a fresh Vite scaffold would.

If the generated repo fights the rebuild, scrap it and scaffold fresh:

```bash
cd /Users/ama/SuperCPAP
npm create vite@latest app -- --template react-ts
```

Then copy only:

- `ai-studio-package/context/04-ui-style-guide.md` as design reference.
- `docs/superpowers/specs/2026-04-27-aerie-cpap-suite-design.md` as product reference.
- Useful type definitions after review.
