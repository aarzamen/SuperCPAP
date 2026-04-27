# Aerie Tauri Local Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Aerie as a local-only macOS Tauri app that analyzes CPAP EDF/CRC files without uploading sensitive data.

**Architecture:** Use React + TypeScript for the instrument-style UI and Rust/Tauri commands for local file selection, EDF parsing, session grouping, metrics, and findings. The app has no web server, no cloud auth, no upload endpoint, and no LLM dependency in v1.

**Tech Stack:** Tauri 2, Rust 2021, React, TypeScript, Vite, npm, Vitest for frontend tests, Rust unit tests for parser/metrics, native macOS file dialogs through Tauri plugins. Current pinned baseline is Rust `tauri` crate `2.10.3`, npm `@tauri-apps/api` and `@tauri-apps/cli` `2.10.1`.

---

## Task 0: Project Guardrails And Scaffold

**Files:**
- Create: `/Users/ama/SuperCPAP/AGENTS.md`
- Create: `/Users/ama/SuperCPAP/app`
- Create: `/Users/ama/SuperCPAP/app/script/build_and_run.sh`
- Create: `/Users/ama/SuperCPAP/.codex/environments/environment.toml`

- [x] Initialize git at `/Users/ama/SuperCPAP` if needed.
- [x] Scaffold a fresh Tauri 2 React TypeScript app:

```bash
cd /Users/ama/SuperCPAP
npm create tauri-app@latest app -- --template react-ts --manager npm --identifier com.aarzamen.aerie --tauri-version 2 --yes
```

- [x] Install dependencies:

```bash
cd /Users/ama/SuperCPAP/app
npm install
```

- [x] Add a project-local run script that kills any prior Aerie process and starts Tauri dev.
- [x] Add Codex environment config so the Run action points to `cd app && ./script/build_and_run.sh`.
- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app
npm run build
```

Expected: frontend TypeScript and Vite build complete.

## Task 1: Local Privacy Baseline

**Files:**
- Create: `/Users/ama/SuperCPAP/app/script/check_privacy_boundary.sh`
- Modify: `/Users/ama/SuperCPAP/app/package.json`

- [x] Add a script that fails if app source contains forbidden cloud/upload patterns:

```bash
cd /Users/ama/SuperCPAP/app
./script/check_privacy_boundary.sh
```

Forbidden patterns for v1 include `firebase`, `oauth`, `cloud run`, `/api/analyze`, `/api/explain`, `serviceWorker`, `navigator.sendBeacon`, and `gtag`.

- [x] Add `privacy:check` and `check` npm scripts.
- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app
npm run privacy:check
```

Expected: pass on the fresh app.

## Task 2: Native Local File Command

**Files:**
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/Cargo.toml`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/lib.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/commands.rs`
- Modify: `/Users/ama/SuperCPAP/app/package.json`
- Modify: `/Users/ama/SuperCPAP/app/src/App.tsx`

- [x] Add Tauri dialog support and a command that accepts selected local file and folder paths.
- [x] Recursively scan selected folders, including multiple folders at once.
- [x] Deduplicate overlapping selections by canonical local file path without returning those paths to React.
- [x] Return a sanitized source summary to React: file count, extensions, byte totals, and accepted/rejected status.
- [x] Do not read or return raw patient/header fields in this task.
- [x] Verify with Rust tests and a local UI smoke run.

## Task 3: EDF Parser TDD

**Files:**
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/mod.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/edf.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/types.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/fixtures.rs`

- [x] Write failing Rust tests for fixed-header parsing, signal metadata, digital-to-physical scaling, and incomplete/header-only files.
- [x] Implement EDF header parsing by byte offset.
- [x] Implement per-record little-endian int16 sample decoding.
- [x] Implement physical scaling:

```text
physical = physical_min + (digital - digital_min) * (physical_max - physical_min) / (digital_max - digital_min)
```

- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app/src-tauri
cargo test analysis::edf
```

Expected: parser tests pass.

## Task 4: Real Sample Golden Checks

**Files:**
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/local_samples.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/edf.rs`

- [x] Add ignored/skippable tests that read the real local sample path only when files exist.
- [x] Assert BRP/PLD/SAD grouping facts and sample-derived ranges.
- [x] Assert SAD SpO2 is unavailable, not a valid oxygen metric.
- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app/src-tauri
cargo test local_sample -- --ignored
```

Expected: pass on this machine; skip or remain ignored elsewhere.

## Task 4a: Local Fixture Quality Profile

**Files:**
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/source_profile.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/mod.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/commands.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/lib.rs`
- Modify: `/Users/ama/SuperCPAP/app/src/App.tsx`
- Modify: `/Users/ama/SuperCPAP/app/src/App.css`
- Modify: `/Users/ama/SuperCPAP/AGENTS.md`

- [x] Add a local-only source quality profile that recursively scans selected files/folders without returning full paths to React.
- [x] Count EDF/CRC/support/rejection totals and valid/limited EDF parsing status.
- [x] Group complete BRP/PLD/SAD sessions when starts are within two seconds.
- [x] Select the longest complete session as the recommended local test fixture.
- [x] Mark Desktop `Untitled Folder 2` as the primary ignored local fixture when present, without copying any data into the repo.
- [x] Surface fixture strengths and limitations in the UI, especially invalid/unavailable SAD oximetry.

## Task 5: Metrics And Findings

**Files:**
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/session.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/metrics.rs`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/findings.rs`

- [x] Write tests for pressure, leak, oximetry-unavailable, incomplete-file quality, and conservative finding language.
- [x] Compute metrics only from decoded samples.
- [x] Produce `AnalysisResult` JSON suitable for React.
- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app/src-tauri
cargo test analysis
```

Expected: all analysis tests pass.

## Task 6: Aerie UI Shell

**Files:**
- Modify: `/Users/ama/SuperCPAP/app/src/App.tsx`
- Create: `/Users/ama/SuperCPAP/app/src/styles.css`
- Create: `/Users/ama/SuperCPAP/app/src/types/analysis.ts`
- Create: `/Users/ama/SuperCPAP/app/src/components/`

- [x] Rebuild the UI from the Claude standalone direction.
- [x] Screens: scope note, file selection, data quality, readout, evidence, lab/future.
- [x] Copy must say processing stays on this Mac.
- [x] Charts must render only real sample-derived summaries.
- [x] Verify:

```bash
cd /Users/ama/SuperCPAP/app
npm run build
```

Expected: no TypeScript errors.

## Task 6a: Lab Feature Queue

**Files:**
- Modify: `/Users/ama/SuperCPAP/app/src/App.tsx`
- Modify: `/Users/ama/SuperCPAP/app/src/App.css`
- Create: `/Users/ama/SuperCPAP/app/src-tauri/src/analysis/lab.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/commands.rs`
- Modify: `/Users/ama/SuperCPAP/app/src-tauri/src/lib.rs`
- Modify: `/Users/ama/SuperCPAP/AGENTS.md`

- [x] Make Lab visible as a featured advanced analysis queue rather than a vague disabled future panel.
- [x] Include breath morphology, trigger/cycle synchrony, leak-pressure interaction, oximetry coupling, instability windows, and counterfactual titration sandbox as gated Lab cards.
- [x] Each Lab card must show status, required signals, and validation posture.
- [x] Copy must say these are exploratory engineering probes and not device-setting recommendations.
- [x] Move the Lab feature catalog into Rust and expose it through a Tauri command.

## Task 7: Final Local Verification

**Files:**
- Modify: `/Users/ama/SuperCPAP/app/README.md`

- [x] Run:

```bash
cd /Users/ama/SuperCPAP/app
npm run check
cd /Users/ama/SuperCPAP/app/src-tauri
cargo test
```

- [x] Run:

```bash
cd /Users/ama/SuperCPAP/app
./script/build_and_run.sh --verify
```

Expected: Aerie launches locally as a Tauri desktop app.

## Stop Conditions

Pause and report clearly if:

- Tauri scaffold fails due to missing macOS system dependencies.
- Rust build fails on WebKit/Tauri dependencies.
- The real sample files are absent from the expected local path.
- A planned feature would require sending CPAP files off-device.
