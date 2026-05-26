import type { ReactNode } from "react";

interface SmallCapsProps {
  children: ReactNode;
  className?: string;
  size?: number;
}

export default function SmallCaps({
  children,
  className = "",
  size = 10.5,
}: SmallCapsProps) {
  return (
    <span
      className={`font-sans uppercase font-medium text-ink-muted ${className}`}
      style={{ fontSize: `${size}px`, letterSpacing: "0.6px" }}
    >
      {children}
    </span>
  );
}
