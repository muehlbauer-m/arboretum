import { useState, useEffect, useCallback, useRef } from "react";
import {
  RefreshCw,
  FileText,
  AlertCircle,
  Loader,
  Search,
  FolderOpen,
  Clock,
} from "lucide-react";
import { listNewsletters, getConfig } from "../lib/api";
import NewsletterViewer from "../components/NewsletterViewer";
import Serif from "../components/Serif";
import SmallCaps from "../components/SmallCaps";
import type { NewsletterMeta } from "../lib/types";

export default function History() {
  const [newsletters, setNewsletters] = useState<NewsletterMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<NewsletterMeta | null>(null);
  const [search, setSearch] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const hasInitialSelection = useRef(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const cfg = await getConfig();
      setOutputDir(cfg.output_dir);
      const items = await listNewsletters(cfg.output_dir);
      setNewsletters(items);
      if (items.length > 0 && !hasInitialSelection.current) {
        setSelected(items[0]);
        hasInitialSelection.current = true;
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const filtered = newsletters.filter(
    (n) =>
      n.title.toLowerCase().includes(search.toLowerCase()) ||
      n.date.includes(search)
  );

  return (
    <div className="h-full flex overflow-hidden bg-canvas">
      {/* List pane */}
      <aside className="w-[280px] shrink-0 flex flex-col border-r border-rule bg-surface-raised">
        {/* Header */}
        <div className="px-4 py-4 border-b border-rule-soft">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Clock size={14} className="text-pine" strokeWidth={1.7} />
              <Serif as="h2" size={15} weight={500}>
                History
              </Serif>
            </div>
            <button
              onClick={load}
              disabled={loading}
              title="Refresh"
              className="p-1.5 rounded-md text-ink-muted hover:text-ink-soft hover:bg-surface transition-colors"
            >
              <RefreshCw
                size={13}
                className={loading ? "animate-spin text-pine" : ""}
              />
            </button>
          </div>

          <div className="relative">
            <Search
              size={12}
              className="absolute left-2.5 top-1/2 -translate-y-1/2 text-ink-faint"
            />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search…"
              className="input pl-8 py-1.5 text-[12.5px]"
            />
          </div>
        </div>

        {/* List */}
        <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
          {loading && (
            <div className="flex items-center justify-center py-8 gap-2 text-ink-faint">
              <Loader size={16} className="animate-spin" />
              <span className="text-sm">Loading…</span>
            </div>
          )}

          {!loading && error && (
            <div className="flex items-start gap-2 p-3 text-rust bg-rust/10 rounded-card mx-1 text-xs">
              <AlertCircle size={14} className="shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          {!loading && !error && filtered.length === 0 && (
            <div className="flex flex-col items-center justify-center py-12 gap-2 text-ink-faint">
              <FileText size={26} strokeWidth={1.4} />
              <p className="text-xs text-center">
                {search ? "No matches found" : "No newsletters yet"}
              </p>
            </div>
          )}

          {!loading &&
            filtered.map((item) => {
              const active = selected?.path === item.path;
              return (
                <button
                  key={item.path}
                  onClick={() => setSelected(item)}
                  className={`
                    relative w-full text-left px-3 py-2.5 rounded-md transition-colors duration-150
                    ${
                      active
                        ? "bg-surface"
                        : "hover:bg-surface/70 border border-transparent"
                    }
                  `}
                >
                  {active && (
                    <span className="absolute left-0 top-1.5 bottom-1.5 w-[3px] bg-pine rounded-r" />
                  )}
                  <Serif
                    as="div"
                    size={13.5}
                    weight={500}
                    className={`leading-[1.25] mb-1 line-clamp-2 ${
                      active ? "text-ink" : "text-ink-soft"
                    }`}
                  >
                    {item.title}
                  </Serif>
                  <div className="flex items-center gap-1.5 text-[10px] text-ink-faint">
                    <span>{item.date}</span>
                    <span>·</span>
                    <span>{item.size_kb} KB</span>
                  </div>
                </button>
              );
            })}
        </div>

        {/* Output dir footer */}
        <div className="px-4 py-2 border-t border-rule-soft flex items-center gap-1.5 text-[10px] text-ink-faint">
          <FolderOpen size={11} strokeWidth={1.5} />
          <span className="truncate" title={outputDir}>
            {outputDir || "~/Documents/newsletters"}
          </span>
        </div>
      </aside>

      {/* Viewer */}
      <div className="flex-1 overflow-hidden">
        {selected ? (
          <NewsletterViewer meta={selected} />
        ) : (
          <div className="h-full flex flex-col items-center justify-center gap-4 text-ink-faint">
            <div className="w-16 h-16 rounded-card bg-surface border border-rule flex items-center justify-center">
              <FileText size={26} strokeWidth={1.4} />
            </div>
            <div className="text-center">
              <SmallCaps className="text-ink-muted">No selection</SmallCaps>
              <p className="mt-2 text-sm text-ink-muted">
                Choose a newsletter from the list.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
