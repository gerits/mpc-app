/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        akai: {
          red: "#e52521",
          redDark: "#a01411",
          redBright: "#ff3a36",
        },
        chassis: {
          900: "#0d0f12",
          850: "#121418",
          800: "#181b21",
          750: "#1e222a",
          700: "#262b35",
          600: "#343a47",
          500: "#495163",
        },
        oled: {
          bg: "#0a120e",
          green: "#2cf583",
          greenDim: "#16633b",
          amber: "#f5a623",
        },
        pad: {
          rubber: "#2a2d34",
          rubberActive: "#3a3f4a",
          glowRed: "rgba(229, 37, 33, 0.6)",
          glowAmber: "rgba(245, 166, 35, 0.6)",
          glowCyan: "rgba(34, 211, 238, 0.6)",
        },
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "ui-monospace", "monospace"],
        sans: ["Inter", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
      },
    },
  },
  plugins: [],
};
