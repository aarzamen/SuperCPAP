import type { ReactNode } from "react";
import { useEffect, useId, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  buildMetricRangeModel,
  buildSourceQualitySegments,
  buildStatusGlyphModel,
  evidenceMeterFromLine,
  formatVisualValue,
} from "./visualModel";
import type { EvidenceMeterModel, StatusGlyphStatus } from "./visualModel";
import "./App.css";

type SourceEntry = {
  fileName: string;
  extension: string;
  byteCount: number;
  accepted: boolean;
  reason?: string | null;
};

type SourceSummary = {
  totalFiles: number;
  acceptedFiles: number;
  rejectedFiles: number;
  totalAcceptedBytes: number;
  entries: SourceEntry[];
};

type FixtureStatus = "strong" | "partial" | "weak";

type RoleCounts = {
  brp: number;
  pld: number;
  sad: number;
  eve: number;
  unknown: number;
};

type OximetrySummary = {
  validSadFiles: number;
  sentinelOnlySadFiles: number;
};

type BestSessionFiles = {
  brp: string;
  pld: string;
  sad: string;
};

type BestSession = {
  startDate: string;
  startTime: string;
  durationSeconds: number;
  files: BestSessionFiles;
  signals: string[];
  limitations: string[];
};

type FixtureRecommendation = {
  status: FixtureStatus;
  title: string;
  summary: string;
};

type SourceQualityProfile = {
  totalFiles: number;
  supportedFiles: number;
  rejectedFiles: number;
  edfFiles: number;
  crcFiles: number;
  csvFiles: number;
  tsvFiles: number;
  validEdfFiles: number;
  limitedEdfFiles: number;
  parseErrorEdfFiles: number;
  roleCounts: RoleCounts;
  validRoleCounts: RoleCounts;
  completeSessions: number;
  oximetry: OximetrySummary;
  bestSession?: BestSession | null;
  recommendation: FixtureRecommendation;
  strengths: string[];
  limitations: string[];
};

type AnalysisStatus = "ready" | "limited" | "empty";

type FindingTone = "evidence" | "review" | "limit";

type SignalStats = {
  channel: string;
  unit: string;
  samples: number;
  min: number;
  median: number;
  p95: number;
  max: number;
  mean: number;
};

type AnalysisOximetryMetrics = {
  spo2?: SignalStats | null;
  pulse?: SignalStats | null;
  unavailableReason?: string | null;
};

type SessionMetrics = {
  pressure?: SignalStats | null;
  leak?: SignalStats | null;
  flow?: SignalStats | null;
  oximetry: AnalysisOximetryMetrics;
};

type Finding = {
  tone: FindingTone;
  title: string;
  body: string;
  evidence: string[];
};

type LabProbeStatus = "available" | "limited" | "gated";

type LabProbeResult = {
  id: string;
  title: string;
  status: LabProbeStatus;
  summary: string;
  evidence: string[];
  limitations: string[];
};

type SessionAnalysis = {
  startDate: string;
  startTime: string;
  durationSeconds: number;
  files: BestSessionFiles;
  metrics: SessionMetrics;
  findings: Finding[];
  labProbes: LabProbeResult[];
};

type AnalysisResult = {
  status: AnalysisStatus;
  sessions: SessionAnalysis[];
  findings: Finding[];
  limitations: string[];
};

type LabFeatureStatus = "queued" | "near" | "gated" | "locked";

type LabFeature = {
  id: string;
  title: string;
  status: LabFeatureStatus;
  signalRequirements: string[];
  validationPosture: string;
  note: string;
  clinicalBoundary: string;
};

const GLOSSARY = {
  CPAP: {
    full: "Continuous Positive Airway Pressure",
    definition: "A PAP mode that holds one pressure level through the breathing cycle.",
  },
  PAP: {
    full: "Positive Airway Pressure",
    definition: "The broad therapy category that includes CPAP, bilevel modes, and related devices.",
  },
  EDF: {
    full: "European Data Format",
    definition: "A binary signal-file format often used for physiologic waveform exports.",
  },
  CRC: {
    full: "Cyclic Redundancy Check",
    definition: "A checksum used to detect whether a file or data block appears corrupted.",
  },
  BRP: {
    full: "Breath and respiratory parameter file",
    definition: "The ResMed-style export role this app treats as the high-rate breath/flow signal source.",
  },
  PLD: {
    full: "Pressure and leak data file",
    definition: "The export role this app treats as pressure and leak trend data.",
  },
  SAD: {
    full: "Saturation and auxiliary data file",
    definition: "The export role this app treats as the SpO2 and pulse signal source when values are valid.",
  },
  EVE: {
    full: "Event data file",
    definition: "The export role that can contain device-scored event markers.",
  },
  SpO2: {
    full: "Peripheral oxygen saturation",
    definition: "The pulse-oximetry estimate of oxygen saturation; invalid sentinels are gated out here.",
  },
  bpm: {
    full: "Beats per minute",
    definition: "A rate unit used for pulse and similar count-per-minute signals.",
  },
  p95: {
    full: "95th percentile",
    definition: "The value that 95 percent of samples are at or below.",
  },
  CV: {
    full: "Coefficient of variation",
    definition: "A normalized variability measure: standard deviation divided by mean.",
  },
  Hz: {
    full: "Hertz",
    definition: "Samples or cycles per second.",
  },
  ms: {
    full: "Milliseconds",
    definition: "One-thousandths of a second; used here in channel sampling labels.",
  },
  TrigCycEvt: {
    full: "Trigger and cycle event",
    definition: "A device event channel used as an exploratory timing marker for synchrony checks.",
  },
  cmH2O: {
    full: "Centimeters of water",
    definition: "A pressure unit commonly used for PAP device pressure.",
  },
  "L/sec": {
    full: "Liters per second",
    definition: "A flow or leak-rate unit used in decoded respiratory signals.",
  },
} as const;

type GlossaryKey = keyof typeof GLOSSARY;

const GLOSSARY_PATTERN =
  /(TrigCycEvt|cmH2O|L\/sec|CPAP|PAP|EDF|CRC|BRP|PLD|SAD|EVE|SpO2|bpm|p95|CV|Hz|ms)/g;

const sampleChecks: { id: string; content: ReactNode }[] = [
  {
    id: "edf-records",
    content: (
      <>
        <GlossaryTerm termKey="EDF" /> headers and sample records parsed locally
      </>
    ),
  },
  {
    id: "physical-scaling",
    content: "Digital samples scaled to physical units before metrics",
  },
  {
    id: "spo2-sentinels",
    content: (
      <>
        Invalid <GlossaryTerm termKey="SpO2" /> sentinels marked unavailable
      </>
    ),
  },
  {
    id: "discussion-support",
    content: "Titration language limited to discussion support",
  },
];

const stages = [
  { label: "Scope", state: "ready" },
  { label: "Files", state: "next" },
  { label: "Quality", state: "locked" },
  { label: "Readout", state: "locked" },
  { label: "Evidence", state: "locked" },
];

type SelectionMode = "files" | "folders";

function isGlossaryKey(value: string): value is GlossaryKey {
  return Object.prototype.hasOwnProperty.call(GLOSSARY, value);
}

function GlossaryTerm({
  termKey,
  children,
}: {
  termKey: GlossaryKey;
  children?: ReactNode;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const popoverId = useId();
  const entry = GLOSSARY[termKey];

  return (
    <span className={`gloss-wrap ${isOpen ? "open" : ""}`}>
      <button
        aria-describedby={popoverId}
        aria-expanded={isOpen}
        aria-label={`${termKey}: ${entry.full}`}
        className="gloss-term"
        onBlur={() => setIsOpen(false)}
        onClick={(event) => {
          event.stopPropagation();
          setIsOpen((current) => !current);
        }}
        type="button"
      >
        {children ?? termKey}
      </button>
      <span className="gloss-popover" id={popoverId} role="tooltip">
        <span className="gloss-full">{entry.full}</span>
        <span className="gloss-definition">{entry.definition}</span>
      </span>
    </span>
  );
}

function GlossaryText({ text }: { text: string }) {
  return (
    <>
      {text.split(GLOSSARY_PATTERN).map((part, index) =>
        isGlossaryKey(part) ? (
          <GlossaryTerm key={`${part}-${index}`} termKey={part} />
        ) : (
          <span key={`${part}-${index}`}>{part}</span>
        ),
      )}
    </>
  );
}

type MetricTone = "pressure" | "leak" | "flow" | "oximetry" | "neutral";

function StatusGlyph({
  status,
  label,
}: {
  status: StatusGlyphStatus;
  label: string;
}) {
  const model = buildStatusGlyphModel(status);
  const dashArray =
    model.lineStyle === "dashed" ? "5 5" : model.lineStyle === "dotted" ? "1 4" : undefined;

  return (
    <svg
      aria-label={`${label}: ${status}`}
      className={`status-glyph ${status}`}
      role="img"
      viewBox="0 0 38 24"
    >
      <line
        className="status-glyph-line"
        strokeDasharray={dashArray}
        strokeLinecap="round"
        strokeWidth={model.strokeWidth}
        x1="5"
        x2="32"
        y1="12"
        y2="12"
      />
      {model.marker === "barred" ? (
        <g className="status-glyph-marker">
          <circle cx="20" cy="12" r="4.5" />
          <line strokeWidth="1.5" x1="16.5" x2="23.5" y1="15.5" y2="8.5" />
        </g>
      ) : (
        <circle
          className={`status-glyph-marker ${model.marker}`}
          cx="20"
          cy="12"
          r={model.marker === "filled" ? 4.5 : 4}
        />
      )}
    </svg>
  );
}

function SourceQualityBar({ profile }: { profile: SourceQualityProfile }) {
  const segments = buildSourceQualitySegments({
    totalFiles: profile.totalFiles,
    acceptedFiles: profile.supportedFiles,
    rejectedFiles: profile.rejectedFiles,
    validEdfFiles: profile.validEdfFiles,
    limitedEdfFiles: profile.limitedEdfFiles,
    parseErrorEdfFiles: profile.parseErrorEdfFiles,
  });

  if (segments.length === 0) {
    return null;
  }

  return (
    <div className="source-viz" aria-label="Source file quality map">
      <div className="source-viz-topline">
        <span className="mono">file map</span>
        <span>{profile.supportedFiles} supported / {profile.totalFiles} total</span>
      </div>
      <div className="stacked-bar" role="img" aria-label="Accepted, limited, parse-error, and rejected file proportions">
        {segments.map((segment) => (
          <span
            className={`stacked-segment ${segment.kind}`}
            key={segment.kind}
            style={{ flexBasis: `${Math.max(segment.percent, 2)}%` }}
            title={`${segment.label}: ${segment.value} files (${segment.percent}%)`}
          />
        ))}
      </div>
      <div className="source-viz-legend">
        {segments.map((segment) => (
          <span key={segment.kind}>
            <i className={`legend-swatch ${segment.kind}`} />
            {segment.label}: {segment.value} ({segment.percent}%)
          </span>
        ))}
      </div>
    </div>
  );
}

function RoleCompletenessMatrix({ profile }: { profile: SourceQualityProfile }) {
  const roles = [
    { key: "brp", label: <GlossaryTerm termKey="BRP" />, total: profile.roleCounts.brp, valid: profile.validRoleCounts.brp },
    { key: "pld", label: <GlossaryTerm termKey="PLD" />, total: profile.roleCounts.pld, valid: profile.validRoleCounts.pld },
    { key: "sad", label: <GlossaryTerm termKey="SAD" />, total: profile.roleCounts.sad, valid: profile.validRoleCounts.sad },
    { key: "eve", label: <GlossaryTerm termKey="EVE" />, total: profile.roleCounts.eve, valid: profile.validRoleCounts.eve },
  ];

  return (
    <div className="role-matrix" aria-label="Role completeness matrix">
      {roles.map((role) => {
        const percent = role.total > 0 ? Math.round((role.valid / role.total) * 100) : 0;
        return (
          <div className="role-meter" key={role.key}>
            <div className="role-meter-topline">
              <span>{role.label}</span>
              <span>{role.valid}/{role.total}</span>
            </div>
            <span className="role-meter-track">
              <span style={{ width: `${percent}%` }} />
            </span>
          </div>
        );
      })}
    </div>
  );
}

function OximetryQualityStrip({ oximetry }: { oximetry: OximetrySummary }) {
  const total = oximetry.validSadFiles + oximetry.sentinelOnlySadFiles;
  const validPercent = total > 0 ? (oximetry.validSadFiles / total) * 100 : 0;
  const sentinelPercent = total > 0 ? (oximetry.sentinelOnlySadFiles / total) * 100 : 0;

  return (
    <div className="oximetry-quality" aria-label="SAD oximetry file validity">
      <div className="source-viz-topline">
        <span className="mono">oximetry gate</span>
        <span>{total === 0 ? "no SAD oximetry counted" : `${total} SAD file(s)`}</span>
      </div>
      <div className="stacked-bar small" role="img" aria-label="Valid versus sentinel-only SAD oximetry files">
        {total > 0 ? (
          <>
            <span
              className="stacked-segment oximetry-valid"
              style={{ flexBasis: `${Math.max(validPercent, oximetry.validSadFiles > 0 ? 2 : 0)}%` }}
              title={`valid SAD oximetry: ${oximetry.validSadFiles}`}
            />
            <span
              className="stacked-segment sentinel"
              style={{
                flexBasis: `${Math.max(sentinelPercent, oximetry.sentinelOnlySadFiles > 0 ? 2 : 0)}%`,
              }}
              title={`sentinel-only SAD oximetry: ${oximetry.sentinelOnlySadFiles}`}
            />
          </>
        ) : (
          <span className="stacked-segment empty" style={{ flexBasis: "100%" }} />
        )}
      </div>
      <div className="source-viz-legend">
        <span><i className="legend-swatch oximetry-valid" />valid {oximetry.validSadFiles}</span>
        <span><i className="legend-swatch sentinel" />sentinel {oximetry.sentinelOnlySadFiles}</span>
      </div>
    </div>
  );
}

function MetricRangeRail({ stats, tone }: { stats: SignalStats; tone: MetricTone }) {
  const model = buildMetricRangeModel(stats);
  const xFor = (position: number) => 24 + (position / 100) * 272;

  return (
    <div className={`metric-rail ${tone}`}>
      <svg
        aria-label={`${model.axisLabel} range rail`}
        role="img"
        viewBox="0 0 320 78"
      >
        <line className="metric-axis" x1="24" x2="296" y1="34" y2="34" />
        <line
          className="metric-range"
          strokeLinecap="round"
          strokeWidth={model.strokeWidth}
          x1={xFor(model.markers[0].x)}
          x2={xFor(model.markers[model.markers.length - 1].x)}
          y1="34"
          y2="34"
        />
        {model.markers.map((marker) => {
          const x = xFor(marker.x);
          return (
            <g className={`metric-marker ${marker.kind}`} key={marker.kind} transform={`translate(${x} 34)`}>
              <title>{marker.label}</title>
              {marker.kind === "p95" ? (
                <path d="M 0 -6 L 6 5 L -6 5 Z" />
              ) : marker.kind === "mean" ? (
                <path d="M 0 -6 L 6 0 L 0 6 L -6 0 Z" />
              ) : (
                <circle r={marker.kind === "median" ? 5 : 4.4} />
              )}
            </g>
          );
        })}
        <text className="axis-label" x="24" y="67">{formatVisualValue(model.domainMin, stats.unit)}</text>
        <text className="axis-label end" x="296" y="67">{formatVisualValue(model.domainMax, stats.unit)}</text>
        <text className="axis-title" x="160" y="14">{model.axisLabel}</text>
      </svg>
      <div className="marker-legend">
        {model.markers.map((marker) => (
          <span className={`marker-label ${marker.kind}`} key={marker.kind}>
            {marker.label}
          </span>
        ))}
      </div>
    </div>
  );
}

function UnavailableRail({ label }: { label: string }) {
  return (
    <div className="unavailable-rail" aria-label={`${label} unavailable`}>
      <StatusGlyph label={label} status="gated" />
      <span>no usable decoded signal</span>
    </div>
  );
}

function SessionTimelineRibbon({ sessions }: { sessions: SessionAnalysis[] }) {
  if (sessions.length === 0) {
    return null;
  }

  const maxDuration = Math.max(...sessions.map((session) => session.durationSeconds), 1);

  return (
    <div className="session-ribbon" aria-label="Analyzed session duration ribbon">
      <div className="source-viz-topline">
        <span className="mono">session ribbon</span>
        <span>{sessions.length} complete session{sessions.length === 1 ? "" : "s"}</span>
      </div>
      <div className="session-ribbon-track">
        {sessions.map((session, index) => {
          const weight = Math.max(1, session.durationSeconds);
          const isLatest = index === sessions.length - 1;
          return (
            <span
              className={isLatest ? "latest" : undefined}
              key={`${session.startDate}-${session.startTime}-${index}`}
              style={{ flexBasis: 0, flexGrow: weight }}
              title={`${session.startDate} ${session.startTime}: ${formatDuration(session.durationSeconds)}`}
            />
          );
        })}
      </div>
      <div className="axis-caption">
        <span>shorter</span>
        <span>longest {formatDuration(maxDuration)}</span>
      </div>
    </div>
  );
}

function FindingCards({ findings }: { findings: Finding[] }) {
  if (findings.length === 0) {
    return null;
  }

  return (
    <div className="finding-cards" aria-label="Decoded findings">
      {findings.map((finding, index) => (
        <FindingCard finding={finding} key={`${finding.title}-${index}`} />
      ))}
    </div>
  );
}

function FindingCard({ finding }: { finding: Finding }) {
  const status = statusFromFindingTone(finding.tone);

  return (
    <article className={`finding-card ${finding.tone}`}>
      <div className="finding-card-topline">
        <StatusGlyph label={finding.title} status={status} />
        <div>
          <span className="mono">{finding.tone}</span>
          <strong>{finding.title}</strong>
        </div>
      </div>
      <p><GlossaryText text={finding.body} /></p>
      {finding.evidence.length > 0 ? (
        <div className="evidence-chip-row">
          {finding.evidence.map((item) => (
            <span className="evidence-chip" key={item}>
              <GlossaryText text={item} />
            </span>
          ))}
        </div>
      ) : null}
    </article>
  );
}

function LabProbeVisual({
  feature,
  probe,
}: {
  feature: LabFeature;
  probe?: LabProbeResult;
}) {
  const status = probe ? statusFromProbeStatus(probe.status) : statusFromFeatureStatus(feature.status);
  const meters = evidenceMeters(probe?.evidence ?? []);

  return (
    <div className={`lab-visual ${status}`}>
      <div className="lab-visual-symbol">
        <StatusGlyph label={feature.title} status={status} />
        <span>{symbolForLabFeature(feature.id)}</span>
      </div>
      {meters.length > 0 ? (
        <div className="evidence-meter-list">
          {meters.slice(0, 3).map((meter) => (
            <EvidenceMeterBar meter={meter} key={`${meter.label}-${meter.valueLabel}`} />
          ))}
        </div>
      ) : (
        <div className="lab-visual-placeholder">
          <span />
          <span />
          <span />
        </div>
      )}
    </div>
  );
}

function EvidenceMeterBar({ meter }: { meter: EvidenceMeterModel }) {
  return (
    <div className="evidence-meter">
      <div>
        <span>{meter.label}</span>
        <strong>{meter.valueLabel}</strong>
      </div>
      <span className="evidence-meter-track">
        <span style={{ width: `${meter.percent}%` }} />
      </span>
    </div>
  );
}

function statusFromFindingTone(tone: FindingTone): StatusGlyphStatus {
  if (tone === "evidence") {
    return "available";
  }
  if (tone === "review") {
    return "limited";
  }
  return "gated";
}

function statusFromProbeStatus(status: LabProbeStatus): StatusGlyphStatus {
  return status;
}

function statusFromFeatureStatus(status: LabFeatureStatus): StatusGlyphStatus {
  return status;
}

function metricTone(title: string): MetricTone {
  const normalized = title.toLowerCase();
  if (normalized.includes("pressure")) {
    return "pressure";
  }
  if (normalized.includes("leak")) {
    return "leak";
  }
  if (normalized.includes("flow")) {
    return "flow";
  }
  if (normalized.includes("ox")) {
    return "oximetry";
  }
  return "neutral";
}

function symbolForLabFeature(id: string): string {
  if (id === "breath_morphology") {
    return "~ I:E";
  }
  if (id === "trigger_cycle_synchrony") {
    return "0|tick";
  }
  if (id === "leak_pressure_interaction") {
    return "L/P r";
  }
  if (id === "oximetry_coupling") {
    return "SpO2 +/-";
  }
  if (id === "instability_windows") {
    return "[CV]|||";
  }
  if (id === "counterfactual_sandbox") {
    return "[low] [wide]";
  }
  return "probe";
}

function evidenceMeters(lines: string[]): EvidenceMeterModel[] {
  return lines
    .map((line) => evidenceMeterFromLine(line))
    .filter((meter): meter is EvidenceMeterModel => Boolean(meter));
}

function App() {
  const [sourceSummary, setSourceSummary] = useState<SourceSummary | null>(null);
  const [sourceProfile, setSourceProfile] = useState<SourceQualityProfile | null>(null);
  const [analysisResult, setAnalysisResult] = useState<AnalysisResult | null>(null);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [labFeatures, setLabFeatures] = useState<LabFeature[]>([]);
  const [labError, setLabError] = useState<string | null>(null);
  const latestSession =
    analysisResult && analysisResult.sessions.length > 0
      ? analysisResult.sessions[analysisResult.sessions.length - 1]
      : null;

  useEffect(() => {
    let active = true;

    invoke<LabFeature[]>("get_lab_features")
      .then((features) => {
        if (active) {
          setLabFeatures(features);
        }
      })
      .catch((error) => {
        if (active) {
          setLabError(error instanceof Error ? error.message : String(error));
        }
      });

    return () => {
      active = false;
    };
  }, []);

  async function selectLocalSources(mode: SelectionMode) {
    setIsSelecting(true);
    setSourceError(null);
    setSourceProfile(null);
    setAnalysisResult(null);

    try {
      const selection =
        mode === "files"
          ? await open({
              multiple: true,
              directory: false,
              filters: [
                {
                  name: "CPAP data",
                  extensions: ["edf", "crc", "csv", "tsv"],
                },
              ],
            })
          : await open({
              multiple: true,
              directory: true,
              title: "Select CPAP data folders",
            });
      const paths = Array.isArray(selection) ? selection : selection ? [selection] : [];

      if (paths.length === 0) {
        return;
      }

      const [summary, profile, analysis] = await Promise.all([
        invoke<SourceSummary>("summarize_source_paths", { paths }),
        invoke<SourceQualityProfile>("profile_source_paths", { paths }),
        invoke<AnalysisResult>("analyze_source_paths", { paths }),
      ]);
      setSourceSummary(summary);
      setSourceProfile(profile);
      setAnalysisResult(analysis);
    } catch (error) {
      setSourceError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSelecting(false);
    }
  }

  return (
    <main className="aerie-shell" aria-label="Aerie local CPAP analysis suite">
      <section className="hero-panel">
        <div className="topline">
          <span className="mono">aerie.</span>
          <span className="status-pill">local macOS build</span>
        </div>

        <div className="hero-copy">
          <p className="eyebrow">
            <GlossaryTerm termKey="CPAP" />/<GlossaryTerm termKey="PAP" /> engineering review
          </p>
          <h1>Inspect therapy data without sending files off this Mac.</h1>
          <p className="lede">
            Aerie is being rebuilt as a desktop instrument: native file selection,
            deterministic <GlossaryTerm termKey="EDF" /> decoding, visible data quality, and conservative
            clinician-discussion support.
          </p>
        </div>

        <div className="cta-row">
          <button
            className="primary-action"
            type="button"
            onClick={() => selectLocalSources("files")}
            disabled={isSelecting}
          >
            {isSelecting ? "Waiting for Finder" : "Select files"}
          </button>
          <button
            className="secondary-action"
            type="button"
            onClick={() => selectLocalSources("folders")}
            disabled={isSelecting}
          >
            Select folders
          </button>
          <span className="hint">
            Folders are scanned recursively; full local paths stay in Rust.
          </span>
        </div>
      </section>

      <section className="workflow-panel" aria-label="Build workflow">
        <div className="section-heading">
          <span className="mono">00</span>
          <h2>Local source status</h2>
        </div>

        {sourceSummary ? (
          <div className="source-summary">
            <div className="summary-number">
              <span className="mono">accepted</span>
              <strong>{sourceSummary.acceptedFiles}</strong>
              <span>of {sourceSummary.totalFiles} files</span>
            </div>
            <div className="summary-meta">
              <span>{formatBytes(sourceSummary.totalAcceptedBytes)}</span>
              <span>{sourceSummary.rejectedFiles} rejected</span>
            </div>
            {sourceProfile ? (
              <SourceProfileCard profile={sourceProfile} />
            ) : (
              <p className="profile-pending">Profiling EDF sessions locally...</p>
            )}
            <ul className="source-list">
              {sourceSummary.entries.slice(0, 5).map((entry) => (
                <li
                  className={entry.accepted ? "accepted" : "rejected"}
                  key={`${entry.fileName}-${entry.byteCount}-${entry.reason ?? "ok"}`}
                >
                  <span>{entry.fileName}</span>
                  <span className="mono">
                    {entry.accepted ? formatBytes(entry.byteCount) : entry.reason}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <div className="stage-grid">
            {stages.map((stage, index) => (
              <div className={`stage-card ${stage.state}`} key={stage.label}>
                <span className="stage-index mono">{String(index + 1).padStart(2, "0")}</span>
                <span>{stage.label}</span>
              </div>
            ))}
          </div>
        )}

        {sourceError ? <p className="source-error">{sourceError}</p> : null}
      </section>

      <section className="evidence-panel" aria-label="Analysis guardrails">
        <div className="section-heading">
          <span className="mono">01</span>
          <h2>Analyzer guardrails</h2>
        </div>

        <ol className="check-list">
          {sampleChecks.map((item) => (
            <li key={item.id}>
              <span className="check-mark" aria-hidden="true" />
              <span>{item.content}</span>
            </li>
          ))}
        </ol>
      </section>

      <section className="readout-panel" aria-label="Decoded readout">
        <div className="section-heading">
          <span className="mono">02</span>
          <h2>Decoded readout</h2>
        </div>

        {analysisResult ? (
          <AnalysisReadout result={analysisResult} />
        ) : (
          <div className="readout-empty">
            <span className="mono">waiting</span>
            <p>Select a folder to compute sample-derived session metrics.</p>
          </div>
        )}
      </section>

      <section className="lab-panel" aria-label="Future lab">
        <div className="section-heading lab-heading">
          <span className="mono">03</span>
          <div>
            <p className="eyebrow">Lab</p>
            <h2>Clever layer, explicitly gated.</h2>
          </div>
        </div>

        <p className="lab-intro">
          Exploratory engineering probes can live here, but every card has to earn its
          way through signal requirements and validation before it influences the readout.
        </p>

        <div className="lab-grid">
          {labFeatures.map((feature) => (
            <LabFeatureCard
              feature={feature}
              key={feature.id}
              probe={latestSession?.labProbes.find((probe) => probe.id === feature.id)}
            />
          ))}
        </div>

        {labError ? <p className="source-error">{labError}</p> : null}

        <div className="lab-boundary">
          <p className="eyebrow">Lab</p>
          <p>
            Lab output is framed as hypothesis, artifact check, or signal probe. It does not
            prescribe <GlossaryTerm termKey="CPAP" /> settings.
          </p>
        </div>
      </section>
    </main>
  );
}

function LabFeatureCard({
  feature,
  probe,
}: {
  feature: LabFeature;
  probe?: LabProbeResult;
}) {
  return (
    <article className="lab-card">
      <div className="lab-card-topline">
        <h3>{feature.title}</h3>
        <span className={`lab-status ${feature.status}`}>{feature.status}</span>
      </div>
      <LabProbeVisual feature={feature} probe={probe} />
      <p className="mono lab-signal">
        {feature.signalRequirements.map((requirement, index) => (
          <span key={`${requirement}-${index}`}>
            {index > 0 ? " + " : ""}
            <GlossaryText text={requirement} />
          </span>
        ))}
      </p>
      <p>
        <GlossaryText text={feature.note} />
      </p>

      {probe ? (
        <div className={`lab-probe ${probe.status}`}>
          <div className="lab-probe-topline">
            <span className="mono">probe</span>
            <span className={`lab-probe-status ${probe.status}`}>{probe.status}</span>
          </div>
          <p>
            <GlossaryText text={probe.summary} />
          </p>
          {probe.evidence.length > 0 ? (
            <ul>
              {probe.evidence.slice(0, 3).map((item) => (
                <li key={item}>
                  <GlossaryText text={item} />
                </li>
              ))}
            </ul>
          ) : null}
          {probe.limitations.length > 0 ? (
            <p className="lab-probe-limit">
              <GlossaryText text={probe.limitations[0]} />
            </p>
          ) : null}
        </div>
      ) : null}

      <p className="lab-validation">
        <GlossaryText text={feature.validationPosture} />
      </p>
    </article>
  );
}

function formatBytes(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function SourceProfileCard({ profile }: { profile: SourceQualityProfile }) {
  return (
    <article className={`fixture-card ${profile.recommendation.status}`}>
      <div className="fixture-topline">
        <div>
          <p className="eyebrow">Fixture quality</p>
          <h3>{profile.recommendation.title}</h3>
        </div>
        <span className={`fixture-status ${profile.recommendation.status}`}>
          {profile.recommendation.status}
        </span>
      </div>
      <p>{profile.recommendation.summary}</p>
      <SourceQualityBar profile={profile} />

      <div className="quality-grid" aria-label="Source quality counts">
        <QualityStat label={<GlossaryTerm termKey="EDF" />} value={profile.edfFiles} />
        <QualityStat label={<GlossaryTerm termKey="CRC" />} value={profile.crcFiles} />
        <QualityStat
          label={
            <>
              Valid <GlossaryTerm termKey="EDF" />
            </>
          }
          value={profile.validEdfFiles}
        />
        <QualityStat label="Sessions" value={profile.completeSessions} />
      </div>

      <RoleCompletenessMatrix profile={profile} />
      <OximetryQualityStrip oximetry={profile.oximetry} />

      {profile.bestSession ? <BestSessionCard session={profile.bestSession} /> : null}

      <FindingList title="Strengths" items={profile.strengths} tone="strength" />
      <FindingList title="Limits" items={profile.limitations} tone="limit" />
    </article>
  );
}

function QualityStat({ label, value }: { label: ReactNode; value: number }) {
  return (
    <div className="quality-stat">
      <span className="mono">{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function BestSessionCard({ session }: { session: BestSession }) {
  return (
    <div className="best-session">
      <div>
        <span className="mono">Best local test session</span>
        <strong>
          {session.startDate} {session.startTime}
        </strong>
        <span>{formatDuration(session.durationSeconds)}</span>
      </div>
      <div className="session-files">
        <span>{session.files.brp}</span>
        <span>{session.files.pld}</span>
        <span>{session.files.sad}</span>
      </div>
      <div className="signal-strip">
        {session.signals.map((signal) => (
          <span key={signal}>
            <GlossaryText text={signal} />
          </span>
        ))}
      </div>
      {session.limitations.length > 0 ? (
        <div className="session-limitations">
          {session.limitations.map((limitation) => (
            <span key={limitation}>
              <GlossaryText text={limitation} />
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function FindingList({
  title,
  items,
  tone,
}: {
  title: string;
  items: string[];
  tone: "strength" | "limit";
}) {
  if (items.length === 0) {
    return null;
  }

  return (
    <div className={`finding-list ${tone}`}>
      <span className="mono">{title}</span>
      <ul>
        {items.slice(0, 3).map((item) => (
          <li key={item}>
            <GlossaryText text={item} />
          </li>
        ))}
      </ul>
    </div>
  );
}

function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  if (hours > 0 && minutes > 0) {
    return `${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h`;
  }
  return `${minutes}m`;
}

function AnalysisReadout({ result }: { result: AnalysisResult }) {
  const session = result.sessions.length > 0 ? result.sessions[result.sessions.length - 1] : null;

  return (
    <div className={`analysis-readout ${result.status}`}>
      <div className="readout-status">
        <span className="mono">{result.status}</span>
        <strong>{result.sessions.length}</strong>
        <span>complete session{result.sessions.length === 1 ? "" : "s"} analyzed</span>
      </div>
      <SessionTimelineRibbon sessions={result.sessions} />

      {session ? (
        <article className="session-readout">
          <div className="session-readout-topline">
            <div>
              <p className="eyebrow">Latest decoded session</p>
              <h3>
                {session.startDate} {session.startTime}
              </h3>
            </div>
            <span className="mono">{formatDuration(session.durationSeconds)}</span>
          </div>

          <div className="metric-grid">
            <MetricCard title="Pressure" stats={session.metrics.pressure} />
            <MetricCard title="Leak" stats={session.metrics.leak} />
            <MetricCard title="Flow" stats={session.metrics.flow} />
          </div>

          <div className="oximetry-card">
            <span className="mono">Oximetry</span>
            {session.metrics.oximetry.spo2 ? (
              <div className="oximetry-rails">
                <MetricRangeRail stats={session.metrics.oximetry.spo2} tone="oximetry" />
                {session.metrics.oximetry.pulse ? (
                  <MetricRangeRail stats={session.metrics.oximetry.pulse} tone="oximetry" />
                ) : null}
              </div>
            ) : (
              <>
                <UnavailableRail label="Oximetry" />
                <p>{session.metrics.oximetry.unavailableReason ?? "No usable oximetry decoded."}</p>
              </>
            )}
          </div>

          <FindingCards findings={session.findings} />
        </article>
      ) : null}

      <FindingList title="Limits" items={result.limitations} tone="limit" />
    </div>
  );
}

function MetricCard({ title, stats }: { title: string; stats?: SignalStats | null }) {
  if (!stats) {
    return (
      <div className="metric-card unavailable">
        <span className="mono">{title}</span>
        <strong>--</strong>
        <UnavailableRail label={title} />
        <span>unavailable</span>
      </div>
    );
  }

  return (
    <div className="metric-card">
      <span className="mono">{title}</span>
      <strong>
        {formatMetric(stats.median)} <small>{stats.unit}</small>
      </strong>
      <MetricRangeRail stats={stats} tone={metricTone(title)} />
      <div className="metric-stat-row">
        <span>mean {formatVisualValue(stats.mean, stats.unit)}</span>
        <span>{stats.samples.toLocaleString()} decoded samples</span>
      </div>
      <span>
        <GlossaryTerm termKey="p95" /> {formatMetric(stats.p95)} · max {formatMetric(stats.max)}
      </span>
    </div>
  );
}

function formatMetric(value: number) {
  if (Math.abs(value) >= 10) {
    return value.toFixed(1);
  }
  return value.toFixed(2);
}

export default App;
