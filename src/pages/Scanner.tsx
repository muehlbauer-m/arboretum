import {
  Radar,
  AlertTriangle,
  Plus,
  CheckCircle,
  Loader,
} from "lucide-react";
import ProfileCard from "../components/ProfileCard";
import OverlapCard from "../components/OverlapCard";
import ProgressLog from "../components/ProgressLog";
import WoodRings from "../components/WoodRings";
import Serif from "../components/Serif";
import SmallCaps from "../components/SmallCaps";
import { useConflict } from "../lib/ConflictContext";

export default function Scanner() {
  const {
    profiles,
    results,
    scanning,
    log,
    addProfile,
    updateProfile,
    removeProfile,
    toggleProfile,
    handleScan,
  } = useConflict();

  const enabledCount = profiles.filter((p) => p.enabled).length;

  const allOverlaps = results
    .flatMap((r) => r.overlaps)
    .sort((a, b) => b.overlap_score - a.overlap_score);

  const scanRanNoOverlaps =
    results.length > 0 && allOverlaps.length === 0 && !scanning;

  return (
    <div className="h-full flex flex-col overflow-hidden bg-canvas">
      {/* Hero */}
      <div className="relative px-10 pt-10 pb-6 shrink-0 border-b border-rule-soft overflow-hidden">
        <div className="absolute right-[-60px] top-[-30px] text-pine">
          <WoodRings
            width={420}
            height={260}
            cx={210}
            cy={130}
            rings={16}
            opacity={0.05}
            seed={7}
          />
        </div>
        <div className="relative flex items-start justify-between gap-6">
          <div>
            <SmallCaps>Overlap scanner</SmallCaps>
            <Serif
              as="h1"
              italic
              weight={400}
              size={32}
              className="mt-2 leading-[1.1]"
            >
              Watch the canopy.
            </Serif>
            <p className="mt-2.5 max-w-[480px] text-[13.5px] text-ink-muted leading-[1.55]">
              Monitor recent papers for overlap with your active research
              profiles before a competing paper eclipses your own.
            </p>
          </div>

          <button
            onClick={handleScan}
            disabled={scanning || enabledCount === 0}
            className="btn-primary text-sm whitespace-nowrap"
            style={{ minWidth: 140 }}
          >
            {scanning ? (
              <>
                <Loader size={15} className="animate-spin" />
                Scanning…
              </>
            ) : (
              <>
                <Radar size={15} />
                Scan now
              </>
            )}
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-10 py-6 space-y-5">
        {(scanning || log.length > 0) && (
          <ProgressLog entries={log} running={scanning} />
        )}

        {/* Profiles */}
        {profiles.length > 0 && (
          <section>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 rounded-full border border-pine" />
                <Serif size={16} weight={500}>
                  My research profiles
                </Serif>
                <span className="text-[10px] text-ink-faint">
                  / {String(profiles.length).padStart(2, "0")}
                </span>
              </div>
              <button
                onClick={addProfile}
                disabled={scanning}
                className="btn-ghost text-[12px] py-1"
              >
                <Plus size={13} />
                Add profile
              </button>
            </div>

            <div className="space-y-2.5">
              {profiles.map((profile) => (
                <ProfileCard
                  key={profile.id}
                  profile={profile}
                  onUpdate={updateProfile}
                  onRemove={removeProfile}
                  onToggle={toggleProfile}
                  disabled={scanning}
                />
              ))}
            </div>
          </section>
        )}

        {/* Overlap alerts */}
        {allOverlaps.length > 0 && (
          <section className="space-y-2.5 animate-fade-in">
            <div className="flex items-center gap-2">
              <AlertTriangle size={14} className="text-amber" strokeWidth={1.6} />
              <Serif size={16} weight={500}>
                Overlap alerts
              </Serif>
              <span className="text-[10px] text-ink-faint">
                / {String(allOverlaps.length).padStart(2, "0")}
              </span>
            </div>
            {allOverlaps.map((overlap, i) => (
              <OverlapCard key={i} overlap={overlap} />
            ))}
          </section>
        )}

        {/* Empty state */}
        {profiles.length === 0 && (
          <div className="flex flex-col items-center justify-center py-20 animate-fade-in">
            <div className="w-16 h-16 rounded-card bg-surface border border-rule flex items-center justify-center mb-4">
              <Radar size={26} className="text-ink-faint" strokeWidth={1.4} />
            </div>
            <Serif size={16} weight={500} className="mb-1">
              No research profiles yet
            </Serif>
            <p className="text-[12.5px] text-ink-muted mb-5">
              Add a profile to start scanning for overlapping papers.
            </p>
            <button
              onClick={addProfile}
              disabled={scanning}
              className="btn-primary text-sm"
            >
              <Plus size={15} />
              Add your first profile
            </button>
          </div>
        )}

        {/* Clean scan result */}
        {scanRanNoOverlaps && (
          <div className="card flex items-center gap-3 border-sage bg-sage-soft animate-fade-in">
            <CheckCircle size={18} className="text-success shrink-0" />
            <div>
              <Serif size={14} weight={500}>
                No overlapping papers found
              </Serif>
              <p className="text-[12px] text-ink-muted mt-0.5">
                No recent papers closely match your active research profiles.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
