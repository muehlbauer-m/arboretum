/**
 * Arboretum's sprout mark — a two-leaf seedling on a stem.
 *
 * This is the same silhouette used for the desktop app icon, so the sidebar
 * "Generate" nav item and the OS dock icon visually agree.
 *
 * Two silhouettes:
 *   - <Sprout/>       — default, legible from 64px up to 1024px
 *   - <SproutSmall/>  — thicker stem, no soil baseline, legible at 16/32px
 */

interface SproutProps {
  size?: number;
  className?: string;
  /** SVG fill — defaults to `currentColor` so callers can tint with Tailwind text color. */
  fill?: string;
}

export function Sprout({
  size = 24,
  className = "",
  fill = "currentColor",
}: SproutProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      className={className}
      aria-hidden="true"
    >
      <g fill={fill}>
        {/* stem */}
        <path d="M48 88 L48 50 Q49 48 52 48 L52 88 Z" />
        {/* soil baseline */}
        <rect x="28" y="86" width="44" height="3" rx="1.5" />
        {/* right leaf */}
        <path d="M50 50 C 50 38, 60 28, 76 26 C 74 38, 66 50, 52 52 Z" />
        {/* left leaf */}
        <path d="M50 56 C 50 46, 42 38, 28 38 C 30 48, 38 56, 50 60 Z" />
      </g>
    </svg>
  );
}

export function SproutSmall({
  size = 16,
  className = "",
  fill = "currentColor",
}: SproutProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      className={className}
      aria-hidden="true"
    >
      <g fill={fill}>
        {/* thicker stem */}
        <path d="M44 92 L44 48 Q46 44 50 44 L56 44 Q58 44 56 48 L56 92 Z" />
        {/* prominent right leaf */}
        <path d="M52 50 C 50 36, 62 22, 84 20 C 82 38, 70 52, 52 54 Z" />
        {/* small left accent leaf */}
        <path d="M48 60 C 46 50, 36 42, 18 42 C 22 56, 34 64, 48 64 Z" />
      </g>
    </svg>
  );
}
