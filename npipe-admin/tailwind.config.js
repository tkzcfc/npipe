/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: ['./index.html', './src/**/*.{vue,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        canvas: 'var(--bg)',
        surface: 'var(--bg-card)',
        sidebar: 'var(--bg-sidebar)',
        ink: 'var(--text)',
        muted: 'var(--text-secondary)',
        faint: 'var(--text-muted)',
        line: 'var(--border)',
        accent: 'var(--accent)',
        'accent-fg': 'var(--accent-fg)',
        'accent-dim': 'var(--accent-bg)',
        danger: 'var(--danger)',
        warning: 'var(--warning)',
        success: 'var(--success)',
      },
      fontFamily: {
        sans: ['IBM Plex Sans', 'PingFang SC', 'Noto Sans SC', 'Microsoft YaHei', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['IBM Plex Mono', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'],
      },
      boxShadow: {
        soft: 'var(--shadow-sm)',
        lift: 'var(--shadow-md)',
      },
      borderRadius: {
        DEFAULT: 'var(--radius)',
      },
    },
  },
  plugins: [],
}
