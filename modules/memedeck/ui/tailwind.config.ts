import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        black: {
          200: "rgba(0, 0, 0, 0.32)",
          400: "#191919",
        },
      },
    },
  },
  plugins: [],
};

export default config;
