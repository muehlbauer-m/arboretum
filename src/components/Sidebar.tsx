import { Clock, Settings as SettingsIcon, Radar, BookOpen, X, Download } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Page } from "../lib/types";
import { SproutSmall } from "./SproutIcon";
import Serif from "./Serif";
import { useLocalPull } from "../lib/LocalPullContext";

interface SidebarProps {
  currentPage: Page;
  onNavigate: (page: Page) => void;
}

interface NavItem {
  page: Page;
  label: string;
  /** Either a Lucide icon component or a custom render fn that receives tint class. */
  Icon?: LucideIcon;
  renderIcon?: (tintClass: string) => JSX.Element;
}

const WORKSPACE: NavItem[] = [
  {
    page: "home",
    label: "Generate",
    renderIcon: (tintClass) => (
      <SproutSmall size={16} className={`${tintClass} shrink-0`} />
    ),
  },
  { page: "history", label: "History", Icon: Clock },
  { page: "scanner", label: "Scanner", Icon: Radar },
];

const SYSTEM: NavItem[] = [
  { page: "settings", label: "Settings", Icon: SettingsIcon },
  { page: "help", label: "Help & guides", Icon: BookOpen },
];

function GroupLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="text-[10px] font-medium uppercase text-ink-faint px-[18px] pb-2"
      style={{ letterSpacing: "0.6px" }}
    >
      {children}
    </div>
  );
}

function NavRow({
  item,
  active,
  onClick,
}: {
  item: NavItem;
  active: boolean;
  onClick: () => void;
}) {
  const tint = active ? "text-pine" : "text-ink-muted";
  return (
    <button
      onClick={onClick}
      className={`
        relative w-full flex items-center gap-3 py-[9px] pl-[18px] pr-[14px]
        text-left
        ${active ? "text-ink" : "text-ink-soft"}
        hover:bg-surface-raised
        transition-colors duration-150
      `}
    >
      {active && (
        <span className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-[18px] bg-pine rounded-r" />
      )}
      {item.renderIcon ? (
        item.renderIcon(tint)
      ) : item.Icon ? (
        <item.Icon size={16} strokeWidth={1.6} className={`shrink-0 ${tint}`} />
      ) : null}
      <span
        className="font-serif"
        style={{ fontSize: "17px", fontWeight: 500, lineHeight: 1.2 }}
      >
        {item.label}
      </span>
    </button>
  );
}

export default function Sidebar({ currentPage, onNavigate }: SidebarProps) {
  return (
    <aside className="w-[220px] shrink-0 flex flex-col bg-canvas border-r border-rule">
      {/* Logo */}
      <div className="flex items-center gap-2.5 pt-5 px-[18px] pb-6">
        <div className="w-[30px] h-[30px] rounded-[7px] bg-pine flex items-center justify-center text-canvas">
          <SproutSmall size={16} className="text-canvas" />
        </div>
        <Serif
          as="span"
          size={22}
          weight={500}
          className="leading-none tracking-tight"
        >
          Arboretum
        </Serif>
      </div>

      {/* Workspace */}
      <GroupLabel>Workspace</GroupLabel>
      {WORKSPACE.map((item) => (
        <NavRow
          key={item.page}
          item={item}
          active={currentPage === item.page}
          onClick={() => onNavigate(item.page)}
        />
      ))}

      <div className="h-[18px]" aria-hidden />

      {/* System */}
      <GroupLabel>System</GroupLabel>
      {SYSTEM.map((item) => (
        <NavRow
          key={item.page}
          item={item}
          active={currentPage === item.page}
          onClick={() => onNavigate(item.page)}
        />
      ))}

      <div className="flex-1" />

      <PullIndicator onNavigate={onNavigate} />

      <div
        className="text-[9px] text-ink-faint font-sans px-[18px] pb-4"
        style={{ letterSpacing: "0.02em" }}
      >
        v0.2 — Apr 2026
      </div>
    </aside>
  );
}

function PullIndicator({ onNavigate }: { onNavigate: (page: Page) => void }) {
  const { activeModel, progress, cancelPull } = useLocalPull();
  if (!activeModel) return null;

  const percent =
    progress?.total && progress.completed
      ? Math.round((progress.completed / progress.total) * 100)
      : null;

  return (
    <div className="mx-[14px] mb-3 px-2.5 py-2 rounded-lg bg-sage-wash border border-moss/40">
      <button
        onClick={() => onNavigate("settings")}
        className="w-full text-left flex items-center gap-1.5 mb-1"
        title="Open Settings"
      >
        <Download size={11} className="text-pine animate-pulse shrink-0" />
        <span className="text-[10.5px] font-medium text-pine-deep truncate">
          Pulling {activeModel}
        </span>
        <button
          onClick={(e) => {
            e.stopPropagation();
            cancelPull();
          }}
          className="ml-auto text-ink-faint hover:text-rust shrink-0"
          title="Cancel pull"
          aria-label="Cancel pull"
        >
          <X size={11} />
        </button>
      </button>
      <div className="w-full bg-rule h-[3px] rounded-full overflow-hidden">
        <div
          className={`h-full bg-pine transition-[width] duration-200 ${
            percent === null ? "animate-pulse w-1/3" : ""
          }`}
          style={percent !== null ? { width: `${percent}%` } : undefined}
        />
      </div>
      <div className="flex justify-between mt-0.5 text-[9px] text-ink-muted">
        <span className="truncate">{progress?.status ?? ""}</span>
        {percent !== null && <span>{percent}%</span>}
      </div>
    </div>
  );
}
