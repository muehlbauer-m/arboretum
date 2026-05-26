interface ArboretumLogoProps {
  size?: number;
  className?: string;
}

/**
 * Compact evergreen silhouette mark. Uses currentColor for the stroke
 * so the caller controls tint via text color.
 */
export default function ArboretumLogo({
  size = 22,
  className = "",
}: ArboretumLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M12 20V9" />
      <path d="M12 9c0-3 2-5 5-5-1 3-2 5-5 5z" />
      <path d="M12 11c0-2-2-4-5-4 1 2 2 4 5 4z" />
    </svg>
  );
}
