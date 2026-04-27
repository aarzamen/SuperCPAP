# Aerie CPAP Suite

Aerie is a local-only macOS desktop app for CPAP/PAP engineering analysis. It is built with Tauri 2, React, TypeScript, and Rust.

The app reads selected EDF/CRC folders on this Mac, groups complete BRP/PLD/SAD sessions, decodes EDF samples, scales digital values into physical units, and displays sample-derived pressure, leak, flow, and oximetry availability. It does not upload CPAP data, call a server, use cloud auth, or generate medical advice.

## Local Privacy Boundary

- CPAP files are processed locally through Rust/Tauri commands.
- Full local file paths stay in Rust and are not displayed in React.
- Real EDF/CRC sample files are not copied into this repository.
- Oximetry sentinel values are reported unavailable rather than summarized as valid oxygenation.
- Readout language is limited to engineering review and clinician-discussion support.

## Run The App

```bash
cd /Users/ama/SuperCPAP/app
./script/build_and_run.sh
```

## Build The macOS App

```bash
cd /Users/ama/SuperCPAP/app
./script/build_and_run.sh --build
```

The built app is written to:

```text
/Users/ama/SuperCPAP/app/src-tauri/target/release/bundle/macos/Aerie.app
```

## Verify The Current Build

Run the standard frontend/privacy/build check:

```bash
cd /Users/ama/SuperCPAP/app
npm run check
```

Run the Rust parser, profiler, metrics, findings, and command tests:

```bash
cd /Users/ama/SuperCPAP/app
cargo test --manifest-path src-tauri/Cargo.toml
```

Run the full desktop smoke check, including bundle build and launch:

```bash
cd /Users/ama/SuperCPAP/app
./script/build_and_run.sh --verify
```

## Optional Local Fixture Checks

These tests read real local CPAP files when present. They are ignored by default and do not copy data into the repo.

```bash
cd /Users/ama/SuperCPAP/app
cargo test --manifest-path src-tauri/Cargo.toml local -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml desktop_fixture -- --ignored --nocapture
```

Expected local fixture path:

```text
/Users/ama/Desktop/Untitled Folder 2
```

That fixture is strong for flow, pressure, leak, session grouping, and invalid-oximetry gating. It is not a valid SpO2/oxygenation fixture because its SAD oximetry decodes as sentinel-only.
