import type { Config } from "tailwindcss";

// Calm, contemporary palette: warm neutral background, muted green/blue
// accents, restrained hues, generous spacing, no gradients or drop shadows.
export default {
  darkMode: ["class"],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: {
          DEFAULT: "#faf8f4",
          dark: "#171512",
        },
        ink: {
          DEFAULT: "#2a2721",
          dark: "#e9e5dc",
        },
        muted: {
          DEFAULT: "#8a8578",
          dark: "#9a968a",
        },
        line: {
          DEFAULT: "#e6e1d6",
          dark: "#332f28",
        },
        surface: {
          DEFAULT: "#ffffff",
          dark: "#1f1c17",
        },
        sage: {
          50: "#f2f6f1",
          100: "#e1eadd",
          200: "#c3d5bc",
          300: "#9fbb93",
          400: "#7ca070",
          500: "#5f8552",
          600: "#4a6941",
          700: "#3c5435",
          800: "#31432c",
          900: "#293825",
        },
        harbor: {
          50: "#f0f4f6",
          100: "#dce6ea",
          200: "#b9ccd5",
          300: "#8fabb9",
          400: "#658a9c",
          500: "#4c7183",
          600: "#3d5b6a",
          700: "#334a56",
          800: "#2c3e48",
          900: "#26343d",
        },
        rust: {
          500: "#b5573a",
          600: "#9c472e",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Inter",
          "system-ui",
          "sans-serif",
        ],
        serif: ["Iowan Old Style", "Georgia", "Charter", "serif"],
      },
      borderRadius: {
        xl: "0.875rem",
        "2xl": "1.25rem",
      },
      boxShadow: {
        soft: "0 1px 2px 0 rgb(0 0 0 / 0.03), 0 1px 6px -1px rgb(0 0 0 / 0.04)",
      },
    },
  },
  plugins: [],
} satisfies Config;
