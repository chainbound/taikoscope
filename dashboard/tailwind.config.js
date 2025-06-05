/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './**/*.{ts,tsx}', '!./node_modules/**/*'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        'taiko-pink': '#e81899',
      },
    },
  },
  plugins: [],
};
