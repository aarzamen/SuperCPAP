import type { ReactNode } from "react";
import { useEffect, useId, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
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

      <div className="role-strip">
        <span>
          <GlossaryTerm termKey="BRP" /> {profile.validRoleCounts.brp}
        </span>
        <span>
          <GlossaryTerm termKey="PLD" /> {profile.validRoleCounts.pld}
        </span>
        <span>
          <GlossaryTerm termKey="SAD" /> {profile.validRoleCounts.sad}
        </span>
        <span>
          <GlossaryTerm termKey="EVE" /> {profile.validRoleCounts.eve}
        </span>
      </div>

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
              <p>
                <GlossaryTerm termKey="SpO2" /> median {formatMetric(session.metrics.oximetry.spo2.median)}{" "}
                {session.metrics.oximetry.spo2.unit}
              </p>
            ) : (
              <p>{session.metrics.oximetry.unavailableReason ?? "No usable oximetry decoded."}</p>
            )}
          </div>

          <FindingList
            title="Findings"
            items={session.findings.map((finding: Finding) => finding.body)}
            tone="strength"
          />
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
