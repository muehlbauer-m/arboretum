import { useMemo, useState } from "react";
import {
  Search,
  Mail,
  Radar,
  CalendarClock,
  Database,
  Folder,
  type LucideIcon,
} from "lucide-react";
import Serif from "../components/Serif";
import SmallCaps from "../components/SmallCaps";
import { SproutSmall } from "../components/SproutIcon";
import WoodRings from "../components/WoodRings";

interface Guide {
  slug: string;
  title: string;
  description: string;
  time: string;
  renderIcon: (tintClass: string) => JSX.Element;
}

function lucide(Icon: LucideIcon) {
  return (tint: string) => (
    <Icon size={15} strokeWidth={1.7} className={tint} />
  );
}

const GUIDES: Guide[] = [
  {
    slug: "getting-started",
    title: "Getting started",
    description: "Generate your first newsletter in under five minutes.",
    time: "3 min read",
    renderIcon: (tint) => <SproutSmall size={15} className={tint} />,
  },
  {
    slug: "email-delivery",
    title: "Set up email delivery",
    description: "Connect Gmail, Outlook, iCloud, or any SMTP host.",
    time: "5 min",
    renderIcon: lucide(Mail),
  },
  {
    slug: "configure-scanner",
    title: "Configure the Scanner",
    description: "Define profiles, set thresholds, read overlap alerts.",
    time: "4 min",
    renderIcon: lucide(Radar),
  },
  {
    slug: "schedule-weekly",
    title: "Schedule weekly runs",
    description:
      "Run Arboretum automatically — Task Scheduler on Windows, launchd on macOS.",
    time: "2 min",
    renderIcon: lucide(CalendarClock),
  },
  {
    slug: "llm-provider",
    title: "Choose an LLM provider",
    description:
      "Anthropic, Gemini, or local models — trade-offs explained.",
    time: "6 min",
    renderIcon: lucide(Database),
  },
  {
    slug: "output-storage",
    title: "Output formats & storage",
    description: "Where files go and how to share them.",
    time: "2 min",
    renderIcon: lucide(Folder),
  },
];

function GuideCard({ guide, onOpen }: { guide: Guide; onOpen: () => void }) {
  return (
    <button
      onClick={onOpen}
      className="
        text-left bg-surface border border-rule rounded-[10px]
        px-[18px] py-4 flex items-start gap-3.5
        hover:border-ink-faint transition-colors duration-150
      "
    >
      <div className="shrink-0 w-8 h-8 rounded-[7px] bg-sage-soft flex items-center justify-center">
        {guide.renderIcon("text-pine")}
      </div>
      <div className="flex-1 min-w-0">
        <Serif as="div" size={16} weight={500} className="mb-0.5">
          {guide.title}
        </Serif>
        <p className="text-[12px] text-ink-muted leading-[1.5] mb-1.5">
          {guide.description}
        </p>
        <SmallCaps size={10} className="text-ink-faint">
          {guide.time}
        </SmallCaps>
      </div>
    </button>
  );
}

export default function Help() {
  const [query, setQuery] = useState("");
  const [opened, setOpened] = useState<string | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return GUIDES;
    return GUIDES.filter(
      (g) =>
        g.title.toLowerCase().includes(q) ||
        g.description.toLowerCase().includes(q)
    );
  }, [query]);

  if (opened) {
    const guide = GUIDES.find((g) => g.slug === opened);
    return (
      <div className="h-full overflow-y-auto bg-canvas">
        <div className="max-w-[720px] mx-auto px-12 py-10">
          <button
            onClick={() => setOpened(null)}
            className="text-[12px] text-pine hover:text-pine-deep underline underline-offset-2 decoration-pine/40 hover:decoration-pine mb-6"
          >
            ← All guides
          </button>
          <SmallCaps className="text-moss">Guide</SmallCaps>
          <Serif
            as="h1"
            italic
            weight={400}
            size={34}
            className="mt-2 mb-5 leading-[1.1]"
          >
            {guide?.title}
          </Serif>
          <p className="text-[14px] text-ink-soft leading-[1.7]">
            This guide hasn't been written yet. When it is, it will live as
            markdown and render through the same prose styles as the newsletter
            viewer.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto bg-canvas">
      {/* Hero */}
      <div className="relative max-w-[920px] mx-auto px-14 pt-10 pb-6 overflow-hidden">
        <div className="absolute right-[-40px] top-[-30px] text-pine">
          <WoodRings
            width={320}
            height={220}
            cx={160}
            cy={110}
            rings={12}
            opacity={0.05}
            seed={11}
          />
        </div>
        <div className="relative">
          <SmallCaps>Help &amp; guides</SmallCaps>
          <Serif
            as="h1"
            italic
            weight={400}
            size={36}
            className="mt-2 leading-[1.1]"
          >
            How can we help?
          </Serif>
          <p className="mt-2.5 max-w-[560px] text-[13.5px] text-ink-muted leading-[1.55]">
            Short guides for the parts of Arboretum that need a little
            explaining.
          </p>
        </div>
      </div>

      {/* Body */}
      <div className="max-w-[920px] mx-auto px-14 pb-14">
        {/* Search */}
        <div className="relative mb-8 max-w-[480px]">
          <Search
            size={14}
            strokeWidth={1.6}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-ink-muted"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search guides"
            className="
              w-full bg-surface border border-rule rounded-lg
              pl-[34px] pr-3 py-2.5 text-[13px] text-ink
              placeholder:text-ink-faint
              focus:border-pine focus:ring-2 focus:ring-pine/20
              outline-none transition-colors
            "
          />
        </div>

        {/* Guide grid */}
        <div>
          <SmallCaps className="mb-3 block">Guides</SmallCaps>
          {filtered.length === 0 ? (
            <p className="text-[13px] text-ink-muted py-8 text-center">
              No guides match "{query}".
            </p>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {filtered.map((g) => (
                <GuideCard
                  key={g.slug}
                  guide={g}
                  onOpen={() => setOpened(g.slug)}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
