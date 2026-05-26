import { useMemo } from "react";

interface WoodRingsProps {
  width?: number;
  height?: number;
  cx?: number;
  cy?: number;
  rings?: number;
  opacity?: number;
  seed?: number;
  className?: string;
  stroke?: string;
}

/**
 * Concentric hand-drawn tree-rings. Purely decorative — absolutely
 * positioned by the caller, never interactive.
 */
export default function WoodRings({
  width = 400,
  height = 400,
  cx = 200,
  cy = 200,
  rings = 14,
  opacity = 0.07,
  seed = 1,
  className = "",
  stroke = "currentColor",
}: WoodRingsProps) {
  const paths = useMemo(() => {
    const rand = (i: number) => {
      const x = Math.sin(i * 12.9898 + seed * 78.233) * 43758.5453;
      return x - Math.floor(x);
    };
    const out: { d: string; w: number }[] = [];
    for (let i = 1; i <= rings; i++) {
      const r = (i / rings) * Math.min(width, height) * 0.7;
      const steps = 60;
      const pts: [number, number][] = [];
      for (let j = 0; j <= steps; j++) {
        const a = (j / steps) * Math.PI * 2;
        const wobble = 1 + (rand(i * 100 + j) - 0.5) * 0.03;
        const rr = r * wobble;
        pts.push([cx + Math.cos(a) * rr, cy + Math.sin(a) * rr * 0.95]);
      }
      const d =
        pts
          .map((p, k) =>
            k === 0
              ? `M${p[0].toFixed(1)},${p[1].toFixed(1)}`
              : `L${p[0].toFixed(1)},${p[1].toFixed(1)}`
          )
          .join(" ") + " Z";
      out.push({ d, w: i % 3 === 0 ? 0.8 : 0.5 });
    }
    return out;
  }, [width, height, cx, cy, rings, seed]);

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={`pointer-events-none absolute inset-0 ${className}`}
      aria-hidden="true"
    >
      {paths.map((p, i) => (
        <path
          key={i}
          d={p.d}
          fill="none"
          stroke={stroke}
          strokeOpacity={opacity}
          strokeWidth={p.w}
        />
      ))}
    </svg>
  );
}
