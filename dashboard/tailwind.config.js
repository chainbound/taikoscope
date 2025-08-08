/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './**/*.{ts,tsx}', '!./node_modules/**/*'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        brand: 'var(--color-brand)',
        bg: 'var(--bg)',
        fg: 'var(--fg)',
        card: 'var(--card)',
        'card-fg': 'var(--card-fg)',
        border: 'var(--border)',
        muted: 'var(--muted)',
        'muted-fg': 'var(--muted-fg)',
        ring: 'var(--ring)',
      },
      // Ensure default border color utilities (e.g., `border`, `border-b`) use our semantic token
      borderColor: {
        DEFAULT: 'var(--border)',
      },
    },
  },
  plugins: [],
};
