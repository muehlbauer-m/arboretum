interface GrowingTreeProps {
  /** 0 – 1 */
  progress: number;
  size?: number;
  className?: string;
}

/**
 * Progress tree. Trunk grows first, branches extend, leaves appear
 * — all as a function of `progress`, no timers.
 */
export default function GrowingTree({
  progress,
  size = 150,
  className = "",
}: GrowingTreeProps) {
  const p = Math.max(0, Math.min(1, progress));
  const trunkP = Math.min(1, p / 0.35);
  const branchP = Math.max(0, Math.min(1, (p - 0.25) / 0.45));
  const leafP = Math.max(0, Math.min(1, (p - 0.55) / 0.45));

  const trunkLen = 90 * trunkP;
  const branches: [number, number, number][] = [
    [70, -35, 36],
    [60, 40, 40],
    [45, -55, 28],
    [40, 25, 32],
    [30, -20, 24],
    [22, 60, 20],
    [15, -45, 18],
  ];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 200 200"
      className={className}
      style={{ overflow: "visible" }}
      aria-hidden="true"
    >
      <ellipse
        cx="100"
        cy="175"
        rx={30 + 40 * p}
        ry={3 + 2 * p}
        fill="currentColor"
        className="text-pine"
        opacity={0.06}
      />

      <line
        x1="100"
        y1="175"
        x2="100"
        y2={175 - trunkLen}
        stroke="currentColor"
        strokeWidth="4"
        strokeLinecap="round"
        className="text-pine"
      />

      {branches.map(([y0, ang, len], i) => {
        const t = Math.max(0, Math.min(1, (branchP - i * 0.05) / 0.4));
        if (t <= 0) return null;
        const rad = (ang * Math.PI) / 180;
        const x1 = 100;
        const y1 = 175 - y0;
        const x2 = x1 + Math.sin(rad) * len * t;
        const y2 = y1 - Math.cos(rad) * len * t;
        return (
          <line
            key={i}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            stroke="currentColor"
            strokeWidth={2}
            strokeLinecap="round"
            opacity={0.85}
            className="text-pine"
          />
        );
      })}

      {branches.map(([y0, ang, len], i) => {
        const delay = i * 0.08;
        const t = Math.max(0, Math.min(1, (leafP - delay) / 0.3));
        if (t <= 0) return null;
        const rad = (ang * Math.PI) / 180;
        const x = 100 + Math.sin(rad) * len;
        const y = 175 - y0 - Math.cos(rad) * len;
        return (
          <g key={`leaf-${i}`} opacity={t} className="text-moss">
            <circle cx={x} cy={y} r={3 + t * 2} fill="currentColor" opacity={0.7} />
            <circle cx={x + 4} cy={y - 2} r={2 + t * 1.5} fill="currentColor" opacity={0.5} />
            <circle cx={x - 3} cy={y + 3} r={2 + t * 1.2} fill="currentColor" opacity={0.6} />
          </g>
        );
      })}
    </svg>
  );
}
