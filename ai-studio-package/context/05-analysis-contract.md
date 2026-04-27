# Aerie Analysis Contract

The deterministic analysis engine emits JSON. The UI and Gemini explanation layer consume this JSON. Gemini must not invent values outside this contract.

## Top-Level Result

```ts
export interface AnalysisResult {
  analysisId: string;
  createdAt: string;
  userVisibleDatasetName: string;
  sourceGroups: SourceGroup[];
  sessions: ParsedSession[];
  aggregate: AggregateSummary;
  findings: Finding[];
  evidence: EvidenceItem[];
  limitations: Limitation[];
  explanationInput: ExplanationInput;
}
```

## Aggregate Summary

```ts
export interface AggregateSummary {
  sessionCount: number;
  totalAnalyzedSeconds: number;
  dateRange?: {
    start: string;
    end: string;
  };
  pressure?: NumericSummary;
  leak?: LeakSummary;
  spo2?: OxygenSummary;
  pulse?: NumericSummary;
  residualEvents?: ResidualEventSummary;
  titrationSupport: {
    supported: boolean;
    direction: "consider_minimum_pressure_discussion" | "focus_on_leak_first" | "insufficient_signal" | "no_change_supported";
    evidenceStrength: "strong" | "moderate" | "weak" | "insufficient";
    summary: string;
    caveats: string[];
  };
}
```

## Source Groups

```ts
export interface SourceGroup {
  id: string;
  label: string;
  uploadedFileCount: number;
  acceptedFileCount: number;
  rejectedFileCount: number;
  totalBytes: number;
  detectedFamily: "resmed_edf_crc" | "oscar_export" | "mixed" | "unknown";
  dateRange?: {
    start: string;
    end: string;
  };
  integrityScore: number;
  warnings: string[];
}
```

## Parsed Session

```ts
export interface ParsedSession {
  id: string;
  startTime?: string;
  durationSeconds: number;
  files: ParsedFileSummary[];
  channels: ChannelSummary[];
  metrics: SessionMetrics;
  quality: DataQuality;
}

export interface DataQuality {
  score: number;
  status: "good" | "limited" | "poor" | "unusable";
  parsedRecordCount: number;
  corruptRecordCount: number;
  missingExpectedChannels: string[];
  warnings: string[];
}

export interface ParsedFileSummary {
  fileName: string;
  role: "brp" | "pld" | "sad" | "eve" | "crc" | "csv" | "unknown";
  format: "edf" | "edf_plus" | "crc" | "csv" | "tsv" | "unsupported";
  valid: boolean;
  recordCount?: number;
  recordDurationSeconds?: number;
  headerBytes?: number;
  warnings: string[];
}
```

## Channels

```ts
export interface ChannelSummary {
  rawLabel: string;
  semantic:
    | "flow"
    | "pressure"
    | "mask_pressure"
    | "epr_pressure"
    | "leak"
    | "resp_rate"
    | "tidal_volume"
    | "minute_ventilation"
    | "ie_ratio"
    | "snore"
    | "flow_limitation"
    | "inspiratory_time"
    | "expiratory_time"
    | "trigger_cycle_event"
    | "pulse"
    | "spo2"
    | "annotation"
    | "crc"
    | "unknown";
  unit: string;
  sampleRateHz: number;
  sampleCount: number;
  physicalMin?: number;
  physicalMax?: number;
  available: boolean;
  caveat?: string;
}
```

## Metrics

```ts
export interface SessionMetrics {
  pressure?: NumericSummary;
  maskPressure?: NumericSummary;
  leak?: LeakSummary;
  flow?: FlowSummary;
  snore?: NumericSummary;
  flowLimitation?: NumericSummary;
  spo2?: OxygenSummary;
  pulse?: NumericSummary;
  residualEvents?: ResidualEventSummary;
}

export interface NumericSummary {
  unit: string;
  min: number;
  median: number;
  p95: number;
  max: number;
  mean?: number;
  unavailableReason?: string;
}

export interface LeakSummary extends NumericSummary {
  highLeakThreshold?: number;
  secondsAboveThreshold?: number;
  highLeakWindows: TimeWindow[];
}

export interface FlowSummary {
  sampleRateHz: number;
  secondsAvailable: number;
  disturbanceWindows: TimeWindow[];
  unavailableReason?: string;
}

export interface OxygenSummary extends NumericSummary {
  secondsBelow90?: number;
  nadir?: number;
  unavailableReason?: string;
}

export interface ResidualEventSummary {
  ahi?: number;
  obstructiveLikePerHour?: number;
  centralLikePerHour?: number;
  hypopneaLikePerHour?: number;
  unavailableReason?: string;
}

export interface TimeWindow {
  startSeconds: number;
  endSeconds: number;
  label: string;
  confidence: "strong" | "moderate" | "weak";
}
```

## Findings

```ts
export interface Finding {
  id: string;
  category:
    | "data_quality"
    | "pressure_support"
    | "leak_support"
    | "flow_limitation_support"
    | "oxygenation_support"
    | "insufficient_signal"
    | "future_lab_unavailable";
  severity: "info" | "attention" | "limited";
  title: string;
  summary: string;
  safeTitrationLanguage?: string;
  evidenceStrength: "strong" | "moderate" | "weak" | "insufficient";
  metricRefs: string[];
  caveats: string[];
}
```

## Evidence

```ts
export interface EvidenceItem {
  id: string;
  findingId?: string;
  label: string;
  value: string;
  source: "device_derived" | "signal_derived" | "literature_derived" | "quality_check" | "insufficient_signal";
  strength: "strong" | "moderate" | "weak" | "insufficient";
  caveat?: string;
}
```

## Limitations

```ts
export interface Limitation {
  id: string;
  scope: "file_quality" | "missing_channel" | "short_duration" | "device_specific" | "clinical_boundary";
  message: string;
  affectedFindings: string[];
}
```

## Explanation Input

```ts
export interface ExplanationInput {
  permittedTone: "clinical_engineering";
  forbiddenClaims: string[];
  userSummaryFacts: string[];
  findingIds: string[];
  evidenceIds: string[];
}
```

Default forbidden claims:

```ts
[
  "diagnosis",
  "prescription",
  "change settings tonight",
  "clinician is wrong",
  "medical advice",
  "condition confirmed"
]
```

## Deterministic Titration Flag Examples

Use conservative output:

- Good: "Pressure data and residual disturbance windows support considering a discussion about minimum pressure."
- Good: "Leak burden limits confidence in pressure interpretation."
- Good: "SpO2 channel is unavailable; oxygenation cannot be assessed from this upload."
- Bad: "Increase EPAP by 0.5 cmH2O."
- Bad: "Your pressure is too low."
