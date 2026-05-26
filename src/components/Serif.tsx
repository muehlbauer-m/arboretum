import type { ElementType, ReactNode, CSSProperties } from "react";

interface SerifProps {
  children: ReactNode;
  as?: ElementType;
  italic?: boolean;
  weight?: 400 | 500 | 600;
  size?: number;
  className?: string;
  style?: CSSProperties;
}

export default function Serif({
  children,
  as: Tag = "span",
  italic = false,
  weight = 500,
  size,
  className = "",
  style,
}: SerifProps) {
  return (
    <Tag
      className={`font-serif text-ink ${italic ? "italic" : ""} ${className}`}
      style={{
        fontWeight: weight,
        lineHeight: 1.15,
        letterSpacing: "-0.005em",
        ...(size ? { fontSize: `${size}px` } : null),
        ...style,
      }}
    >
      {children}
    </Tag>
  );
}
