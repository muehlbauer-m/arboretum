import { useState, useEffect } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { open } from "@tauri-apps/plugin-shell";
import { Loader, AlertCircle, Mail, CheckCircle } from "lucide-react";
import { readNewsletter, sendNewsletterEmail } from "../lib/api";
import type { NewsletterMeta } from "../lib/types";
import Serif from "./Serif";
import SmallCaps from "./SmallCaps";

function openUrl(href: string | undefined) {
  if (!href) return;
  open(href).catch(() => {
    window.open(href, "_blank", "noopener,noreferrer");
  });
}

interface NewsletterViewerProps {
  meta: NewsletterMeta;
}

export default function NewsletterViewer({ meta }: NewsletterViewerProps) {
  const [content, setContent] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    setContent("");
    setSent(false);
    setSendError(null);

    readNewsletter(meta.path)
      .then((text) => setContent(text))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [meta.path]);

  const handleSendEmail = async () => {
    setSending(true);
    setSendError(null);
    setSent(false);
    try {
      await sendNewsletterEmail(meta.path);
      setSent(true);
    } catch (e) {
      setSendError(String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-canvas">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-7 py-4 border-b border-rule-soft bg-surface shrink-0">
        <div className="min-w-0">
          <Serif as="h2" size={17} weight={500} className="truncate max-w-xl">
            {meta.title}
          </Serif>
          <div className="mt-1">
            <SmallCaps size={10}>
              {meta.date} · {meta.size_kb} KB
            </SmallCaps>
          </div>
        </div>

        <div className="flex items-center gap-3 shrink-0">
          {sent && (
            <span className="flex items-center gap-1.5 text-xs text-success">
              <CheckCircle size={13} /> Sent
            </span>
          )}
          <button
            onClick={handleSendEmail}
            disabled={sending || loading}
            className="btn-secondary text-[12.5px] py-1.5 px-3"
          >
            {sending ? (
              <Loader size={13} className="animate-spin" />
            ) : (
              <Mail size={13} />
            )}
            {sending ? "Sending…" : "Send via email"}
          </button>
        </div>
      </div>

      {/* Full error message — visible, not just a tooltip */}
      {sendError && (
        <div className="mx-7 mt-3 flex items-start gap-2 p-3 bg-rust/10 border border-rust/25 rounded-card text-rust text-[12.5px]">
          <AlertCircle size={14} className="shrink-0 mt-0.5" />
          <div className="min-w-0">
            <div className="font-medium mb-0.5">Send failed</div>
            <div className="text-[12px] text-ink-soft break-words">
              {sendError}
            </div>
          </div>
        </div>
      )}

      {/* Reader */}
      <div className="flex-1 overflow-y-auto px-10 py-10 selectable">
        {loading && (
          <div className="flex items-center justify-center h-40 gap-3 text-ink-muted">
            <Loader size={20} className="animate-spin" />
            <span>Loading newsletter…</span>
          </div>
        )}

        {error && (
          <div className="flex items-center gap-3 text-rust bg-rust/10 border border-rust/20 rounded-card p-4">
            <AlertCircle size={18} />
            <span className="text-sm">{error}</span>
          </div>
        )}

        {!loading && !error && content && (
          <article className="prose max-w-[640px] mx-auto">
            <div className="mb-6">
              <SmallCaps className="text-moss">Research digest</SmallCaps>
            </div>
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                a: ({ href, children }) => (
                  <a
                    href={href}
                    onClick={(e) => {
                      e.preventDefault();
                      openUrl(href);
                    }}
                  >
                    {children}
                  </a>
                ),
              }}
            >
              {content}
            </ReactMarkdown>
          </article>
        )}
      </div>
    </div>
  );
}
