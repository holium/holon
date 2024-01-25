import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        black: {
          32: "rgba(0, 0, 0, 0.32)",
          200: "rgba(0, 0, 0, 0.32)",
          400: "#191919",
        },
        white: {
          4: "rgba(255, 255, 255, 0.04)",
          10: "rgba(255, 255, 255, 0.10)",
          32: "rgba(255, 255, 255, 0.32)",
        },
      },
    },
  },
  plugins: [],
};

export default config;
