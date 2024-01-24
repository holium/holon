/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        black: {
          200: 'rgba(0, 0, 0, 0.32)',
          400: '#191919',
        },
      },
    },
  },
  plugins: [],
}
