import { useEffect, useState } from "react";
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

type SessionAnalysis = {
  startDate: string;
  startTime: string;
  durationSeconds: number;
  files: BestSessionFiles;
  metrics: SessionMetrics;
  findings: Finding[];
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

const sampleChecks = [
  "EDF headers and sample records parsed locally",
  "Digital samples scaled to physical units before metrics",
  "Invalid SpO2 sentinels marked unavailable",
  "Titration language limited to discussion support",
];

const stages = [
  { label: "Scope", state: "ready" },
  { label: "Files", state: "next" },
  { label: "Quality", state: "locked" },
  { label: "Readout", state: "locked" },
  { label: "Evidence", state: "locked" },
];

type SelectionMode = "files" | "folders";

function App() {
  const [sourceSummary, setSourceSummary] = useState<SourceSummary | null>(null);
  const [sourceProfile, setSourceProfile] = useState<SourceQualityProfile | null>(null);
  const [analysisResult, setAnalysisResult] = useState<AnalysisResult | null>(null);
  const [sourceError, setSourceError] = useState<string | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [labFeatures, setLabFeatures] = useState<LabFeature[]>([]);
  const [labError, setLabError] = useState<string | null>(null);

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
          <p className="eyebrow">CPAP/PAP engineering review</p>
          <h1>Inspect therapy data without sending files off this Mac.</h1>
          <p className="lede">
            Aerie is being rebuilt as a desktop instrument: native file selection,
            deterministic EDF decoding, visible data quality, and conservative
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
            <li key={item}>
              <span className="check-mark" aria-hidden="true" />
              <span>{item}</span>
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
            <article className="lab-card" key={feature.id}>
              <div className="lab-card-topline">
                <h3>{feature.title}</h3>
                <span className={`lab-status ${feature.status}`}>{feature.status}</span>
              </div>
              <p className="mono lab-signal">{feature.signalRequirements.join(" + ")}</p>
              <p>{feature.note}</p>
              <p className="lab-validation">{feature.validationPosture}</p>
            </article>
          ))}
        </div>

        {labError ? <p className="source-error">{labError}</p> : null}

        <div className="lab-boundary">
          <p className="eyebrow">Lab</p>
          <p>
            Lab output is framed as hypothesis, artifact check, or signal probe. It does not
            prescribe CPAP settings.
          </p>
        </div>
      </section>
    </main>
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
        <QualityStat label="EDF" value={profile.edfFiles} />
        <QualityStat label="CRC" value={profile.crcFiles} />
        <QualityStat label="Valid EDF" value={profile.validEdfFiles} />
        <QualityStat label="Sessions" value={profile.completeSessions} />
      </div>

      <div className="role-strip">
        <span>BRP {profile.validRoleCounts.brp}</span>
        <span>PLD {profile.validRoleCounts.pld}</span>
        <span>SAD {profile.validRoleCounts.sad}</span>
        <span>EVE {profile.validRoleCounts.eve}</span>
      </div>

      {profile.bestSession ? <BestSessionCard session={profile.bestSession} /> : null}

      <FindingList title="Strengths" items={profile.strengths} tone="strength" />
      <FindingList title="Limits" items={profile.limitations} tone="limit" />
    </article>
  );
}

function QualityStat({ label, value }: { label: string; value: number }) {
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
          <span key={signal}>{signal}</span>
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
          <li key={item}>{item}</li>
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
                SpO2 median {formatMetric(session.metrics.oximetry.spo2.median)}{" "}
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
        p95 {formatMetric(stats.p95)} · max {formatMetric(stats.max)}
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
