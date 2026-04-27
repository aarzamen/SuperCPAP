# EDF / CPAP Channel Reference

This reference is based on observed CPAP/PAP EDF files in the sample set:

`/Users/ama/Library/Mobile Documents/com~apple~CloudDocs/syd docs /Sydney PAP 2025 Peninsula Trial/20250914`

The files appear to be ResMed-style EDF/CRC exports with roles such as BRP, PLD, SAD, and EVE.

## EDF Basics

The parser must read:

- version
- patient identification
- recording identification
- start date
- start time
- header byte count
- reserved field
- number of data records
- duration per data record
- number of signals
- per-signal labels
- per-signal units
- physical min/max
- digital min/max
- samples per record

Digital samples are signed little-endian int16 values. Convert to physical values:

```ts
physical = physicalMin + (digital - digitalMin) * (physicalMax - physicalMin) / (digitalMax - digitalMin)
```

## Observed File Roles

### BRP

High-rate breath/pressure signals.

Observed channels:

- `Flow.40ms`
  - semantic: `flow`
  - unit: `L/s`
  - sample interval: 40 ms
  - sample rate: 25 Hz
- `Press.40ms`
  - semantic: `pressure`
  - unit: `cmH2O`
  - sample rate: 25 Hz
- `TrigCycEvt.40ms`
  - semantic: `trigger_cycle_event`
  - unit: none
- `Crc16`
  - semantic: `crc`

### PLD

Lower-rate therapy metrics.

Observed channels:

- `MaskPress.2s` -> `mask_pressure`, `cmH2O`, 0.5 Hz
- `Press.2s` -> `pressure`, `cmH2O`, 0.5 Hz
- `EprPress.2s` -> `epr_pressure`, `cmH2O`, 0.5 Hz
- `Leak.2s` -> `leak`, `L/s`, 0.5 Hz
- `RespRate.2s` -> `resp_rate`, `bpm`, 0.5 Hz
- `TidVol.2s` -> `tidal_volume`, `L`, 0.5 Hz
- `MinVent.2s` -> `minute_ventilation`, `L/min`, 0.5 Hz
- `IERatio.2s` -> `ie_ratio`, `%`, 0.5 Hz
- `Snore.2s` -> `snore`, unitless, 0.5 Hz
- `FlowLim.2s` -> `flow_limitation`, unitless, 0.5 Hz
- `B5ITime.2s` -> `inspiratory_time`, `s`, 0.5 Hz
- `B5ETime.2s` -> `expiratory_time`, `s`, 0.5 Hz
- `Ti.2s` -> `inspiratory_time`, `s`, 0.5 Hz
- `Crc16` -> `crc`

### SAD

Oximetry-style signals when available.

Observed channels:

- `Pulse.1s` -> `pulse`, `bpm`, 1 Hz
- `SpO2.1s` -> `spo2`, `%`, 1 Hz
- `Crc16` -> `crc`

Observed caveat: SpO2 and pulse can exist as channels but contain invalid/missing sentinel-like values. The app must report unavailable or invalid rather than pretending oxygenation is known.

### EVE

Event/annotation file.

Observed channels:

- `EDF Annotations` -> `annotation`
- `Crc16` -> `crc`

Observed annotation:

- "Recording starts"

## Session Grouping

Files commonly share a timestamp prefix:

```text
20250914_211945_BRP.edf
20250914_211945_PLD.edf
20250914_211945_SAD.edf
20250914_211945_BRP.crc
20250914_211945_PLD.crc
20250914_211945_SAD.crc
```

Group by date/time prefix first, then reconcile slight one-second differences between BRP/PLD/SAD start times.

## Header-Only Or Incomplete Files

Some EDF files may have:

- record count `-1`
- no data records beyond header
- very small file size
- only annotations

These should not crash parsing. Mark as limited or incomplete.

## Units

Normalize display units:

- `cmH2O` display as `cmH₂O`.
- `L/s` for flow/leak unless converted.
- `bpm` for pulse/respiratory rate.
- `%` for SpO2, IE ratio, flow limitation display when appropriate.

