import { useState } from "react";
import {
  Pencil,
  X,
  Plus,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import type { CompetitionProfile } from "../lib/types";
import Serif from "./Serif";

interface ProfileCardProps {
  profile: CompetitionProfile;
  onUpdate: (id: string, updates: Partial<CompetitionProfile>) => void;
  onRemove: (id: string) => void;
  onToggle: (id: string) => void;
  disabled: boolean;
}

export default function ProfileCard({
  profile,
  onUpdate,
  onRemove,
  onToggle,
  disabled,
}: ProfileCardProps) {
  const [editing, setEditing] = useState(!profile.name);
  const [draft, setDraft] = useState({
    name: profile.name,
    research_description: profile.research_description,
    key_terms: [...profile.key_terms],
    own_papers: [...profile.own_papers],
  });
  const [termInput, setTermInput] = useState("");
  const [showPapers, setShowPapers] = useState(false);

  const handleSave = () => {
    const name =
      draft.name.trim() ||
      draft.research_description.trim().slice(0, 40) ||
      "Untitled Profile";
    onUpdate(profile.id, {
      name,
      research_description: draft.research_description,
      key_terms: draft.key_terms,
      own_papers: draft.own_papers.filter((p) => p.trim()),
    });
    setEditing(false);
  };

  const handleCancel = () => {
    if (!profile.name && !profile.research_description) {
      onRemove(profile.id);
      return;
    }
    setDraft({
      name: profile.name,
      research_description: profile.research_description,
      key_terms: [...profile.key_terms],
      own_papers: [...profile.own_papers],
    });
    setEditing(false);
  };

  const addTerm = () => {
    const term = termInput.trim();
    if (term && !draft.key_terms.includes(term)) {
      setDraft((d) => ({ ...d, key_terms: [...d.key_terms, term] }));
    }
    setTermInput("");
  };

  const removeTerm = (term: string) => {
    setDraft((d) => ({
      ...d,
      key_terms: d.key_terms.filter((t) => t !== term),
    }));
  };

  const addPaper = () =>
    setDraft((d) => ({ ...d, own_papers: [...d.own_papers, ""] }));

  const updatePaper = (i: number, value: string) =>
    setDraft((d) => {
      const papers = [...d.own_papers];
      papers[i] = value;
      return { ...d, own_papers: papers };
    });

  const removePaper = (i: number) =>
    setDraft((d) => ({
      ...d,
      own_papers: d.own_papers.filter((_, j) => j !== i),
    }));

  const lastScanned = profile.last_scanned
    ? new Date(profile.last_scanned).toLocaleDateString()
    : "never";

  // ── Edit mode ──
  if (editing) {
    return (
      <div className="card animate-fade-in space-y-4">
        {/* Description */}
        <div>
          <label className="label">Research Description</label>
          <textarea
            value={draft.research_description}
            onChange={(e) =>
              setDraft((d) => ({ ...d, research_description: e.target.value }))
            }
            placeholder="Describe your specific research focus, e.g. 'Novel attention mechanisms for efficient long-context transformers applied to genomics.'"
            rows={4}
            disabled={disabled}
            className="input resize-none leading-relaxed"
          />
        </div>

        {/* Key terms */}
        <div>
          <label className="label">Key Terms</label>
          <input
            type="text"
            value={termInput}
            onChange={(e) => setTermInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addTerm();
              }
            }}
            placeholder="Type a term and press Enter"
            disabled={disabled}
            className="input"
          />
          {draft.key_terms.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mt-2">
              {draft.key_terms.map((term) => (
                <span
                  key={term}
                  className="
                    inline-flex items-center gap-1 bg-sage-wash border border-sage
                    text-pine-deep text-[11px] font-medium rounded-full pl-2.5 pr-1 py-0.5
                  "
                >
                  {term}
                  <button
                    onClick={() => removeTerm(term)}
                    disabled={disabled}
                    className="p-0.5 hover:text-rust transition-colors"
                  >
                    <X size={10} />
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Own papers */}
        <div>
          <button
            onClick={() => setShowPapers(!showPapers)}
            className="label flex items-center gap-2 hover:text-ink-soft transition-colors"
            style={{ marginBottom: 0 }}
          >
            Own Papers (optional)
            {showPapers ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          </button>

          {showPapers && (
            <div className="mt-2 space-y-2 animate-fade-in">
              {draft.own_papers.map((paper, i) => (
                <div key={i} className="flex gap-2">
                  <input
                    type="text"
                    value={paper}
                    onChange={(e) => updatePaper(i, e.target.value)}
                    placeholder="DOI or paper title"
                    disabled={disabled}
                    className="input flex-1"
                  />
                  <button
                    onClick={() => removePaper(i)}
                    disabled={disabled}
                    className="p-2 text-ink-faint hover:text-rust transition-colors"
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
              <button
                onClick={addPaper}
                disabled={disabled}
                className="btn-ghost text-xs py-1"
              >
                <Plus size={12} />
                Add Paper
              </button>
            </div>
          )}
        </div>

        {/* Name */}
        <div>
          <label className="label">Profile Name</label>
          <input
            type="text"
            value={draft.name}
            onChange={(e) => setDraft((d) => ({ ...d, name: e.target.value }))}
            placeholder={
              draft.research_description.trim().slice(0, 40) ||
              "Auto-generated from description"
            }
            disabled={disabled}
            className="input"
          />
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-1">
          <button
            onClick={handleSave}
            disabled={disabled}
            className="btn-primary text-xs px-4 py-2"
          >
            Save
          </button>
          <button
            onClick={handleCancel}
            disabled={disabled}
            className="btn-ghost text-xs py-2"
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  // ── View mode ──
  return (
    <div className={`card flex items-start justify-between gap-4 ${profile.enabled ? "" : "opacity-60"}`}>
      <div className="flex-1 min-w-0">
        <Serif as="h3" size={15} weight={500}>
          {profile.name}
        </Serif>
        <p className="text-[12.5px] text-ink-muted mt-1.5 line-clamp-2 leading-[1.5]">
          {profile.research_description}
        </p>
        {profile.key_terms.length > 0 && (
          <div className="flex flex-wrap gap-1.5 mt-2.5">
            {profile.key_terms.slice(0, 4).map((term) => (
              <span
                key={term}
                className="
                  inline-flex items-center bg-sage-wash border border-sage
                  text-pine-deep text-[10.5px] font-medium rounded-full px-2 py-0.5
                "
              >
                {term}
              </span>
            ))}
            {profile.key_terms.length > 4 && (
              <span className="inline-flex items-center bg-sage-soft border border-sage text-pine-deep text-[10.5px] font-medium rounded-full px-2 py-0.5">
                +{profile.key_terms.length - 4}
              </span>
            )}
          </div>
        )}
        <div className="mt-2.5 text-[10px] text-ink-faint flex gap-2">
          <span>{profile.key_terms.length} terms</span>
          <span>·</span>
          <span>{profile.own_papers.length} papers</span>
          <span>·</span>
          <span>last scanned {lastScanned}</span>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {/* Toggle */}
        <button
          onClick={() => !disabled && onToggle(profile.id)}
          disabled={disabled}
          aria-pressed={profile.enabled}
          aria-label={profile.enabled ? "Disable profile" : "Enable profile"}
          className={`
            relative w-9 h-5 p-0 border-0 rounded-full shrink-0
            transition-colors duration-150
            ${profile.enabled ? "bg-pine" : "bg-rule"}
            disabled:opacity-50 disabled:cursor-not-allowed
          `}
        >
          <span
            className="
              absolute top-0.5 w-4 h-4 bg-surface rounded-full shadow
              transition-[left] duration-150
            "
            style={{ left: profile.enabled ? 18 : 2 }}
          />
        </button>

        <button
          onClick={() => {
            setDraft({
              name: profile.name,
              research_description: profile.research_description,
              key_terms: [...profile.key_terms],
              own_papers: [...profile.own_papers],
            });
            setEditing(true);
          }}
          disabled={disabled}
          title="Edit profile"
          className="p-1.5 rounded-md text-ink-faint hover:text-ink-soft hover:bg-surface-raised transition-colors"
        >
          <Pencil size={14} />
        </button>

        <button
          onClick={() => onRemove(profile.id)}
          disabled={disabled}
          title="Remove profile"
          className="p-1.5 rounded-md text-ink-faint hover:text-rust transition-colors"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
