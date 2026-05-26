import { useRef, useEffect } from "react";
import { CheckCircle, XCircle, Info, Loader, Activity } from "lucide-react";
import type { LogEntry } from "../lib/types";
import SmallCaps from "./SmallCaps";

interface ProgressLogProps {
  entries: LogEntry[];
  running: boolean;
}

export default function ProgressLog({ entries, running }: ProgressLogProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries]);

  if (entries.length === 0 && !running) return null;

  return (
    <div className="card animate-fade-in">
      <div className="flex items-center gap-2 mb-3">
        {running ? (
          <Loader size={14} className="text-pine animate-spin" />
        ) : (
          <CheckCircle size={14} className="text-success" />
        )}
        <SmallCaps size={10.5} className="text-ink-soft">
          {running ? "Generating…" : "Complete"}
        </SmallCaps>
        <span className="ml-auto">
          <SmallCaps size={10} className="text-ink-faint">
            {entries.length} events
          </SmallCaps>
        </span>
      </div>

      <div
        className="
          bg-surface-raised border border-rule-soft rounded-lg px-3 py-2
          max-h-48 overflow-y-auto space-y-0.5
        "
      >
        {entries.map((entry, i) => {
          const isLiveStream =
            entry.streamingStep && i === entries.length - 1;
          return (
            <div
              key={i}
              className={`flex items-start gap-2 py-0.5 text-[11.5px] leading-relaxed animate-fade-in ${
                entry.type === "error"
                  ? "text-rust"
                  : entry.type === "success"
                  ? "text-success"
                  : isLiveStream
                  ? "text-pine"
                  : "text-ink-soft"
              }`}
            >
              <span className="shrink-0 mt-0.5">
                {entry.type === "error" ? (
                  <XCircle size={11} />
                ) : entry.type === "success" ? (
                  <CheckCircle size={11} />
                ) : isLiveStream ? (
                  <Activity size={11} className="animate-pulse" />
                ) : (
                  <Info size={11} />
                )}
              </span>

              <span className="text-ink-faint shrink-0">{entry.ts}</span>

              {entry.topic && (
                <span className="text-moss shrink-0 truncate max-w-[120px]">
                  [{entry.topic.slice(0, 14)}
                  {entry.topic.length > 14 ? "…" : ""}]
                </span>
              )}

              <span className="flex-1 break-all">{entry.message}</span>
            </div>
          );
        })}

        {running && (
          <div className="flex items-center gap-2 text-pine py-0.5 text-[11.5px]">
            <Loader size={11} className="animate-spin" />
            <span>Processing…</span>
          </div>
        )}

        <div ref={bottomRef} />
      </div>
    </div>
  );
}
