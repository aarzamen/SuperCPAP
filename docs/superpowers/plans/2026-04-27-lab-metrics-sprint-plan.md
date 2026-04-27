# Aerie Lab Metrics Sprint Plan

**Goal:** Support deterministic calculations behind the Lab section without changing the local-only privacy boundary or making device-setting recommendations.

**Boundary:** All Lab output is exploratory engineering analysis. Results are framed as probes, artifacts, signal quality checks, or hypotheses. No result prescribes CPAP settings.

## Shared Contract

Create a Rust-owned Lab result contract under `app/src-tauri/src/analysis/lab_common.rs`.

Each Lab module should return:

- stable feature id matching `lab.rs`
- status: `available`, `limited`, or `gated`
- short summary
- evidence strings derived from decoded samples
- limitation strings when signals are missing, sentinel-only, too sparse, or otherwise not interpretable

## Parallel Work Lanes

### Lane A: Breath Morphology

**Write scope:** `app/src-tauri/src/analysis/lab_breath.rs`

Calculate from BRP `Flow.40ms`:

- breath count from zero-crossing segmentation
- median breath duration
- inspiratory/expiratory balance estimate
- flattening candidate ratio based on plateau-like inspiratory samples
- unstable breath ratio based on breath-to-breath amplitude variation

### Lane B: Trigger/Cycle Synchrony

**Write scope:** `app/src-tauri/src/analysis/lab_synchrony.rs`

Calculate from BRP `Flow.40ms` and optional `TrigCycEvt.40ms`:

- event availability and event count
- event rate per minute
- whether event samples align to usable flow samples
- gated/limited result when event-code mapping is absent

### Lane C: Leak-Pressure Interaction And Counterfactual Sandbox

**Write scope:** `app/src-tauri/src/analysis/lab_pressure.rs`

Calculate from PLD `Leak.2s`, `Press.2s`, and optional `MaskPress.2s`:

- leak-pressure correlation
- high-leak fraction
- pressure variability during high leak versus baseline leak
- mask-pressure delta when available
- counterfactual confidence bands as engineering hypotheses only

### Lane D: Oximetry Coupling And Instability Windows

**Write scope:** `app/src-tauri/src/analysis/lab_oximetry_instability.rs`

Calculate from valid SAD `SpO2.1s`, `Pulse.1s`, and optional PLD `RespRate.2s` / `MinVent.2s`:

- oximetry coupling availability
- SpO2 drop count and pulse response summary when physiologic oximetry exists
- sentinel-only gated result when oximetry is invalid
- instability windows based on respiratory-rate/minute-ventilation variability when available

## Integration

After workers finish, integrate in the main session:

- expose all Lab probe results through `AnalysisResult`
- add a Lab metrics readout to the UI cards
- keep existing deterministic readout stable
- verify full Rust tests, frontend build, local fixture tests, and desktop launch

## Verification

Run:

```bash
cd /Users/ama/SuperCPAP/app
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml local -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml desktop_fixture -- --ignored --nocapture
./script/build_and_run.sh --verify
```
