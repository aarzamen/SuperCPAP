export type SignalStatsLike = {
  channel: string;
  unit: string;
  samples: number;
  min: number;
  median: number;
  p95: number;
  max: number;
  mean: number;
};

export type MetricMarkerKind = "min" | "mean" | "median" | "p95" | "max";

export type MetricMarker = {
  kind: MetricMarkerKind;
  value: number;
  x: number;
  label: string;
};

export type MetricRangeModel = {
  axisLabel: string;
  domainMin: number;
  domainMax: number;
  strokeWidth: number;
  markers: MetricMarker[];
};

export type SourceQualityInput = {
  totalFiles: number;
  acceptedFiles: number;
  rejectedFiles: number;
  validEdfFiles: number;
  limitedEdfFiles: number;
  parseErrorEdfFiles: number;
};

export type SourceQualitySegmentKind =
  | "valid"
  | "limited"
  | "parse_error"
  | "accepted_other"
  | "rejected";

export type SourceQualitySegment = {
  kind: SourceQualitySegmentKind;
  label: string;
  value: number;
  percent: number;
};

export type StatusGlyphStatus = "available" | "limited" | "gated" | "locked" | "queued" | "near";

export type StatusGlyphModel = {
  lineStyle: "solid" | "dotted" | "dashed";
  strokeWidth: number;
  marker: "filled" | "hollow" | "barred";
};

export type EvidenceMeterModel = {
  label: string;
  valueLabel: string;
  percent: number;
};

export function buildMetricRangeModel(stats: SignalStatsLike): MetricRangeModel {
  const domainMin = finiteOrZero(stats.min);
  const domainMax = finiteOrZero(stats.max);
  const markers: MetricMarker[] = [
    marker("min", stats.min, stats, domainMin, domainMax),
    marker("mean", stats.mean, stats, domainMin, domainMax),
    marker("median", stats.median, stats, domainMin, domainMax),
    marker("p95", stats.p95, stats, domainMin, domainMax),
    marker("max", stats.max, stats, domainMin, domainMax),
  ];

  return {
    axisLabel: `${stats.channel} · ${stats.unit}`,
    domainMin,
    domainMax,
    strokeWidth: strokeWidthForSamples(stats.samples),
    markers,
  };
}

export function buildSourceQualitySegments(input: SourceQualityInput): SourceQualitySegment[] {
  const total = Math.max(0, input.totalFiles);
  if (total === 0) {
    return [];
  }

  const valid = Math.max(0, input.validEdfFiles);
  const limited = Math.max(0, input.limitedEdfFiles);
  const parseError = Math.max(0, input.parseErrorEdfFiles);
  const rejected = Math.max(0, input.rejectedFiles);
  const acceptedOther = Math.max(
    0,
    input.acceptedFiles - valid - limited - parseError,
  );

  return [
    segment("valid", "valid EDF", valid, total),
    segment("limited", "limited EDF", limited, total),
    segment("parse_error", "parse error", parseError, total),
    segment("accepted_other", "other accepted", acceptedOther, total),
    segment("rejected", "rejected", rejected, total),
  ].filter((item) => item.value > 0);
}

export function buildStatusGlyphModel(status: StatusGlyphStatus): StatusGlyphModel {
  if (status === "available" || status === "near") {
    return {
      lineStyle: "solid",
      strokeWidth: status === "available" ? 2.6 : 2.2,
      marker: "filled",
    };
  }

  if (status === "limited" || status === "queued") {
    return {
      lineStyle: "dotted",
      strokeWidth: status === "limited" ? 1.8 : 1.5,
      marker: "hollow",
    };
  }

  return {
    lineStyle: "dashed",
    strokeWidth: 1.2,
    marker: "barred",
  };
}

export function formatVisualValue(value: number, unit: string): string {
  const safeValue = finiteOrZero(value);
  const digits = Math.abs(safeValue) >= 10 ? 1 : 2;
  return `${safeValue.toFixed(digits)} ${unit}`;
}

export function evidenceMeterFromLine(line: string): EvidenceMeterModel | null {
  const percentMatch = line.match(/(-?\d+(?:\.\d+)?)%/);
  if (percentMatch) {
    const value = Number(percentMatch[1]);
    return {
      label: conciseEvidenceLabel(line),
      valueLabel: `${value.toFixed(Math.abs(value) >= 10 ? 1 : 2)}%`,
      percent: clamp(value, 0, 100),
    };
  }

  const correlationMatch = line.match(/correlation:\s*(-?\d+(?:\.\d+)?)/i);
  if (correlationMatch) {
    const value = Number(correlationMatch[1]);
    return {
      label: "correlation",
      valueLabel: `r ${value.toFixed(2)}`,
      percent: clamp(((value + 1) / 2) * 100, 0, 100),
    };
  }

  const countMatch = line.match(
    /(?:^|\b)(?:(\d+(?:\.\d+)?)\s+(?:aligned\s+)?(?:samples|events|windows|breaths)|(?:count|samples|events|windows|breaths)\s*:?\s*(\d+(?:\.\d+)?))/i,
  );
  if (countMatch) {
    const value = Number(countMatch[1] ?? countMatch[2]);
    return {
      label: conciseEvidenceLabel(line),
      valueLabel: value.toLocaleString(),
      percent: roundOne(clamp((Math.log10(value + 1) / 3) * 100, 4, 100)),
    };
  }

  const bandMatch = line.match(/(-?\d+(?:\.\d+)?)-(-?\d+(?:\.\d+)?)\s*(cmH2O|L\/sec|bpm|%|s)/);
  if (bandMatch) {
    return {
      label: conciseEvidenceLabel(line),
      valueLabel: `${bandMatch[1]}-${bandMatch[2]} ${bandMatch[3]}`,
      percent: 72,
    };
  }

  return null;
}

function marker(
  kind: MetricMarkerKind,
  value: number,
  stats: SignalStatsLike,
  domainMin: number,
  domainMax: number,
): MetricMarker {
  return {
    kind,
    value: finiteOrZero(value),
    x: positionPercent(value, domainMin, domainMax),
    label: `${kind} ${formatVisualValue(value, stats.unit)}`,
  };
}

function segment(
  kind: SourceQualitySegmentKind,
  label: string,
  value: number,
  total: number,
): SourceQualitySegment {
  return {
    kind,
    label,
    value,
    percent: roundedPercent(value, total),
  };
}

function positionPercent(value: number, min: number, max: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(min) || !Number.isFinite(max)) {
    return 50;
  }
  if (max === min) {
    return 50;
  }
  return clamp(((value - min) / (max - min)) * 100, 0, 100);
}

function strokeWidthForSamples(samples: number): number {
  if (samples >= 100) {
    return 2.8;
  }
  if (samples >= 30) {
    return 2.2;
  }
  return 1.6;
}

function roundedPercent(value: number, total: number): number {
  if (total <= 0) {
    return 0;
  }
  return roundOne((value / total) * 100);
}

function finiteOrZero(value: number): number {
  return Number.isFinite(value) ? value : 0;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function conciseEvidenceLabel(line: string): string {
  return line.split(":")[0].replace(/\.$/, "").slice(0, 28);
}

function roundOne(value: number): number {
  return Math.round(value * 10) / 10;
}
