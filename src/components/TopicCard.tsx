import { X } from "lucide-react";

interface TopicCardProps {
  id: string;
  query: string;
  index: number;
  onChange: (id: string, value: string) => void;
  onRemove: (id: string) => void;
  disabled?: boolean;
}

const PLACEHOLDERS = [
  "How are transformer models being applied to protein folding?",
  "Recent advances in solid-state lithium batteries",
  "Economics of carbon markets and climate policy",
  "CRISPR gene editing in clinical trials 2024",
  "Large language models for code generation",
  "Quantum computing error correction breakthroughs",
];

export default function TopicCard({
  id,
  query,
  index,
  onChange,
  onRemove,
  disabled = false,
}: TopicCardProps) {
  const placeholder = PLACEHOLDERS[index % PLACEHOLDERS.length];

  return (
    <div className="group flex items-start gap-3 bg-surface border border-rule rounded-card p-3.5 transition-colors duration-150 hover:border-sage">
      {/* Number badge */}
      <div
        className="shrink-0 w-[26px] h-[26px] rounded-md bg-sage-wash border border-sage
                   flex items-center justify-center font-serif text-pine-deep"
        style={{ fontSize: "13px", fontWeight: 500 }}
      >
        {String(index + 1).padStart(2, "0")}
      </div>

      {/* Textarea */}
      <textarea
        value={query}
        onChange={(e) => onChange(id, e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        rows={2}
        className="
          flex-1 min-w-0 bg-transparent border-0 text-ink placeholder:text-ink-faint
          resize-none outline-none p-0 text-sm leading-relaxed
          disabled:opacity-50 disabled:cursor-not-allowed
        "
      />

      {/* Remove */}
      <button
        onClick={() => onRemove(id)}
        disabled={disabled}
        title="Remove topic"
        className="
          shrink-0 p-1.5 rounded-md text-ink-faint hover:text-rust
          transition-colors duration-150
          disabled:opacity-30 disabled:cursor-not-allowed
        "
      >
        <X size={14} strokeWidth={1.8} />
      </button>
    </div>
  );
}
