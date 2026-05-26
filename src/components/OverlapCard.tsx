import type { OverlapResult } from "../lib/types";
import Serif from "./Serif";

interface OverlapCardProps {
  overlap: OverlapResult;
}

type Severity = "high" | "med" | "low";

function classify(score: number): Severity {
  if (score >= 70) return "high";
  if (score >= 40) return "med";
  return "low";
}

const SEVERITY_STYLES: Record<
  Severity,
  { badgeBg: string; badgeText: string; rail: string }
> = {
  high: {
    badgeBg: "bg-rust/10 border-rust/25",
    badgeText: "text-rust",
    rail: "border-l-rust",
  },
  med: {
    badgeBg: "bg-amber/15 border-amber/30",
    badgeText: "text-bark",
    rail: "border-l-amber",
  },
  low: {
    badgeBg: "bg-sage-wash border-sage",
    badgeText: "text-pine-deep",
    rail: "border-l-moss",
  },
};

export default function OverlapCard({ overlap }: OverlapCardProps) {
  const severity = classify(overlap.overlap_score);
  const s = SEVERITY_STYLES[severity];

  return (
    <div
      className={`
        bg-surface border border-rule rounded-card p-4
        flex items-start gap-4 border-l-[3px] ${s.rail}
      `}
    >
      {/* Circular score */}
      <div
        className={`
          shrink-0 w-[54px] h-[54px] rounded-xl border
          flex flex-col items-center justify-center
          ${s.badgeBg}
        `}
      >
        <Serif size={20} weight={500} className={s.badgeText}>
          {overlap.overlap_score}
        </Serif>
        <span
          className={`font-sans uppercase font-semibold ${s.badgeText}`}
          style={{ fontSize: "9px", letterSpacing: "0.7px" }}
        >
          {severity}
        </span>
      </div>

      <div className="flex-1 min-w-0">
        <Serif as="h4" size={15} weight={500}>
          {overlap.paper_title}
        </Serif>
        <div className="mt-1 text-[11px] text-ink-muted">
          {overlap.paper_authors} · {overlap.paper_date} ·{" "}
          {overlap.paper_source}
        </div>

        <p className="text-[12.5px] text-ink-soft mt-2.5 leading-[1.55]">
          {overlap.overlap_explanation}
        </p>

        {overlap.matched_terms.length > 0 && (
          <div className="flex flex-wrap gap-1.5 mt-2.5">
            {overlap.matched_terms.map((term) => (
              <span
                key={term}
                className="
                  inline-flex items-center px-2 py-0.5 rounded-full
                  bg-sage-wash border border-sage text-pine-deep
                  text-[11px] font-medium
                "
              >
                {term}
              </span>
            ))}
          </div>
        )}

        <div className="mt-2.5 text-[11px] text-ink-faint">
          Matched against{" "}
          <span className="text-pine font-medium">{overlap.profile_name}</span>
        </div>
      </div>
    </div>
  );
}
