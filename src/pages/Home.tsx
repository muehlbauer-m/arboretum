import { useMemo } from "react";
import {
  Plus,
  Sprout,
  Database,
  BookOpen,
  CheckCircle,
  XCircle,
  Radar,
  Loader,
} from "lucide-react";
import TopicCard from "../components/TopicCard";
import ProgressLog from "../components/ProgressLog";
import GrowingTree from "../components/GrowingTree";
import WoodRings from "../components/WoodRings";
import Serif from "../components/Serif";
import SmallCaps from "../components/SmallCaps";
import { useGeneration } from "../lib/GenerationContext";
import { useConflict } from "../lib/ConflictContext";

function formatIssue(d: Date): { no: string; month: string } {
  return {
    no: String(d.getMonth() + 1).padStart(2, "0"),
    month: d.toLocaleDateString("en-US", { month: "long", year: "numeric" }),
  };
}

export default function Home() {
  const {
    topics,
    sources,
    maxPapers,
    daysBack,
    running,
    log,
    results,
    addTopic,
    updateTopic,
    removeTopic,
    toggleSource,
    handleGenerate,
    setMaxPapers,
    setDaysBack,
  } = useGeneration();

  const { profiles } = useConflict();
  const enabledProfiles = profiles.filter((p) => p.enabled).length;

  const issue = useMemo(() => formatIssue(new Date()), []);

  // Progress is approximated from log completion ratio so the tree grows
  // meaningfully even without a real step count.
  const totalExpected = Math.max(topics.length * 6, 6);
  const progress = running
    ? Math.min(0.97, log.length / totalExpected)
    : results.length > 0
    ? 1
    : 0;
  const pct = Math.round(progress * 100);

  return (
    <div className="h-full flex flex-col overflow-hidden bg-canvas">
      {/* Hero */}
      <div className="relative px-10 pt-10 pb-6 shrink-0 border-b border-rule-soft overflow-hidden">
        <div className="absolute right-[-80px] top-[-60px] text-pine">
          <WoodRings
            width={360}
            height={300}
            cx={180}
            cy={150}
            rings={16}
            opacity={0.065}
            seed={3}
          />
        </div>

        <div className="relative flex items-start justify-between gap-6">
          <div className="min-w-0">
            <SmallCaps>
              No. {issue.no} · {issue.month}
            </SmallCaps>
            <Serif
              as="h1"
              italic
              weight={400}
              size={36}
              className="mt-2 leading-[1.1]"
            >
              Cultivate a research digest.
            </Serif>
            <p className="mt-2.5 max-w-[460px] text-[13.5px] text-ink-muted leading-[1.55]">
              Enter your interests in plain language. Arboretum fetches recent
              papers from OpenAlex and arXiv, then summarizes the most relevant
              ones into a single reading.
            </p>
          </div>

          <button
            onClick={handleGenerate}
            disabled={running}
            className="btn-primary text-sm whitespace-nowrap"
            style={{ minWidth: 140 }}
          >
            {running ? (
              <>
                <Loader size={15} className="animate-spin" />
                Growing…
              </>
            ) : (
              <>
                <Sprout size={15} />
                Generate
              </>
            )}
          </button>
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto px-10 py-6 space-y-5">
        {/* Generation panel (only during/after a real run) */}
        {(running || results.length > 0) && (
          <div className="card-raised relative overflow-hidden">
            <div className="absolute right-[-40px] top-[-20px] text-pine">
              <WoodRings
                width={400}
                height={220}
                cx={200}
                cy={110}
                rings={14}
                opacity={0.06}
                seed={5}
              />
            </div>
            <div className="relative grid grid-cols-[160px_1fr] gap-5 items-center">
              <div className="flex items-center justify-center border-r border-rule-soft pr-4">
                <GrowingTree progress={progress} size={140} />
              </div>
              <div>
                <div className="flex items-center justify-between mb-1.5">
                  <div>
                    <Serif size={17} weight={500}>
                      {running ? "Growing your digest" : "Digest complete"}
                    </Serif>
                    <p className="text-[12px] text-ink-muted mt-0.5">
                      Fetching, curating, and summarizing papers.
                    </p>
                  </div>
                  <Serif size={22} weight={500} className="text-pine">
                    {pct}%
                  </Serif>
                </div>
                <div className="h-1 bg-rule-soft rounded-full overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-moss to-pine transition-[width] duration-500 ease-soft"
                    style={{ width: `${pct}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Log — always visible when there are entries */}
        {(running || log.length > 0) && (
          <ProgressLog entries={log} running={running} />
        )}

        {/* Topics */}
        <section>
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <BookOpen size={14} className="text-pine" strokeWidth={1.7} />
              <Serif size={16} weight={500}>
                Topics
              </Serif>
              <span className="text-[10px] text-ink-faint">
                / {String(topics.length).padStart(2, "0")}
              </span>
            </div>
            <button
              onClick={addTopic}
              disabled={running}
              className="btn-ghost text-[12px] py-1"
            >
              <Plus size={13} />
              Add topic
            </button>
          </div>

          <div className="space-y-2.5">
            {topics.map((topic, i) => (
              <TopicCard
                key={topic.id}
                id={topic.id}
                query={topic.query}
                index={i}
                onChange={updateTopic}
                onRemove={removeTopic}
                disabled={running}
              />
            ))}
          </div>
        </section>

        {/* Options */}
        <section className="card space-y-5">
          <div className="flex items-center gap-2">
            <div className="w-4 h-4 rounded-full border border-moss" />
            <Serif size={15} weight={500}>
              Options
            </Serif>
          </div>

          <div className="grid grid-cols-2 gap-6">
            {/* Sources */}
            <div>
              <label className="label">Sources</label>
              <div className="flex gap-2">
                {[
                  { id: "openalex", label: "OpenAlex", Icon: Database },
                  { id: "arxiv", label: "arXiv", Icon: BookOpen },
                ].map(({ id, label, Icon }) => {
                  const active = sources.includes(id);
                  return (
                    <button
                      key={id}
                      onClick={() => !running && toggleSource(id)}
                      disabled={running}
                      className={`
                        inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[12.5px] font-medium
                        border transition-colors duration-150
                        ${
                          active
                            ? "bg-sage-wash border-moss text-pine-deep"
                            : "bg-surface border-rule text-ink-muted hover:text-ink-soft hover:border-ink-faint"
                        }
                        disabled:opacity-50 disabled:cursor-not-allowed
                      `}
                    >
                      <Icon size={13} strokeWidth={1.6} />
                      {label}
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Max / days */}
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="label">Papers / source</label>
                <div className="flex items-center gap-2">
                  <input
                    type="range"
                    min={10}
                    max={100}
                    step={10}
                    value={maxPapers}
                    onChange={(e) => setMaxPapers(Number(e.target.value))}
                    disabled={running}
                    className="flex-1 accent-pine"
                  />
                  <span className="w-7 text-right text-[12px] text-ink">
                    {maxPapers}
                  </span>
                </div>
              </div>
              <div>
                <label className="label">Date range</label>
                <select
                  value={daysBack}
                  onChange={(e) => setDaysBack(Number(e.target.value))}
                  disabled={running}
                  className="input py-1.5 text-[12.5px]"
                >
                  {[7, 14, 30, 60, 90, 180, 365].map((d) => (
                    <option key={d} value={d}>
                      Last {d} days
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </div>

          {enabledProfiles > 0 && (
            <div className="pt-3 border-t border-rule-soft flex items-center gap-2 text-[11.5px] text-ink-muted">
              <Radar size={13} className="text-pine" strokeWidth={1.6} />
              <span>
                {enabledProfiles} research profile
                {enabledProfiles === 1 ? "" : "s"} active — use the{" "}
                <span className="text-pine font-medium">Scanner</span> to check
                for overlaps.
              </span>
            </div>
          )}
        </section>

        {/* Results */}
        {results.length > 0 && (
          <section className="space-y-2 animate-fade-in">
            <div className="flex items-center gap-2">
              <CheckCircle size={14} className="text-success" strokeWidth={1.8} />
              <Serif size={15} weight={500}>
                Results
              </Serif>
            </div>
            {results.map((r, i) => (
              <div
                key={i}
                className={`
                  flex items-start gap-3 card
                  ${r.error ? "border-rust/30" : "border-sage"}
                `}
              >
                {r.error ? (
                  <XCircle size={16} className="text-rust mt-0.5 shrink-0" />
                ) : (
                  <CheckCircle
                    size={16}
                    className="text-success mt-0.5 shrink-0"
                  />
                )}
                <div className="min-w-0 flex-1">
                  <Serif size={14} weight={500} className="truncate">
                    {r.title || r.topic}
                  </Serif>
                  {r.error ? (
                    <p className="text-[12px] text-rust mt-0.5">{r.error}</p>
                  ) : (
                    <p className="text-[11.5px] text-ink-muted mt-0.5 truncate">
                      {r.path}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </section>
        )}
      </div>
    </div>
  );
}
