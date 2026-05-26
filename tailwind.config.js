/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        canvas: "rgb(var(--color-canvas) / <alpha-value>)",
        surface: "rgb(var(--color-surface) / <alpha-value>)",
        "surface-raised": "rgb(var(--color-surface-raised) / <alpha-value>)",
        "surface-sunk": "rgb(var(--color-surface-sunk) / <alpha-value>)",
        ink: "rgb(var(--color-ink) / <alpha-value>)",
        "ink-soft": "rgb(var(--color-ink-soft) / <alpha-value>)",
        "ink-muted": "rgb(var(--color-ink-muted) / <alpha-value>)",
        "ink-faint": "rgb(var(--color-ink-faint) / <alpha-value>)",
        rule: "rgb(var(--color-rule) / <alpha-value>)",
        "rule-soft": "rgb(var(--color-rule-soft) / <alpha-value>)",
        pine: "rgb(var(--color-pine) / <alpha-value>)",
        "pine-deep": "rgb(var(--color-pine-deep) / <alpha-value>)",
        moss: "rgb(var(--color-moss) / <alpha-value>)",
        sage: "rgb(var(--color-sage) / <alpha-value>)",
        "sage-wash": "rgb(var(--color-sage-wash) / <alpha-value>)",
        "sage-soft": "rgb(var(--color-sage-soft) / <alpha-value>)",
        bark: "rgb(var(--color-bark) / <alpha-value>)",
        amber: "rgb(var(--color-amber) / <alpha-value>)",
        rust: "rgb(var(--color-rust) / <alpha-value>)",
        success: "rgb(var(--color-success) / <alpha-value>)",
        warning: "rgb(var(--color-warning) / <alpha-value>)",
      },
      fontFamily: {
        serif: ["'Cormorant Garamond'", "Georgia", "serif"],
        sans: ["Inter", "system-ui", "sans-serif"],
      },
      letterSpacing: {
        caps: "0.06em",
      },
      animation: {
        "fade-in": "fadeIn 240ms cubic-bezier(0.2, 0.8, 0.2, 1)",
        "slide-in": "slideIn 240ms cubic-bezier(0.2, 0.8, 0.2, 1)",
        "pulse-slow": "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0", transform: "translateY(6px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        slideIn: {
          "0%": { opacity: "0", transform: "translateX(-12px)" },
          "100%": { opacity: "1", transform: "translateX(0)" },
        },
      },
      transitionTimingFunction: {
        soft: "cubic-bezier(0.2, 0.8, 0.2, 1)",
      },
      borderRadius: {
        card: "12px",
      },
    },
  },
  plugins: [],
};
