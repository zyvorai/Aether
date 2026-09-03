/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        background: 'var(--background)',
        foreground: 'var(--foreground)',
        surface: 'var(--surface)',
        'surface-elevated': 'var(--surface-elevated)',
        primary: 'rgb(var(--primary-rgb) / <alpha-value>)',
        'primary-wash': 'var(--accent-tint)',
        muted: 'var(--muted-foreground)',
        subtle: 'var(--subtle)',
        border: 'var(--border)',
        danger: 'var(--danger)',
        warn: 'var(--warning)',
        success: 'var(--success)',
        hover: 'var(--hover)',
        rule: 'var(--rule)',
        'rule-strong': 'var(--rule-strong)',
        brand: 'rgb(var(--brand-rgb) / <alpha-value>)',
        'brand-wash': 'var(--brand-wash)',
        glass: 'var(--glass)',
        aether: {
          DEFAULT: '#0071e3',
          light: '#0a84ff',
          dark: '#0058b3',
          dim: 'rgba(0, 113, 227, 0.14)',
          glow: 'rgba(0, 113, 227, 0.28)',
        },
        'aether-ai': {
          DEFAULT: '#0071e3',
          light: '#409cff',
          dark: '#0058b3',
          dim: 'rgba(0, 113, 227, 0.14)',
          glow: 'rgba(0, 113, 227, 0.28)',
        },
      },
      fontFamily: {
        sans: ['var(--sans)'],
        display: ['var(--font-display)'],
        mono: ['var(--mono)'],
      },
      boxShadow: {
        ambient: 'var(--shadow)',
        card: 'var(--shadow-card)',
      },
      borderRadius: {
        liquid: 'var(--radius-liquid)',
        pill: 'var(--radius-pill)',
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'scale-in': 'scaleIn 0.2s ease-out',
      },
      keyframes: {
        fadeIn: {
          from: { opacity: '0' },
          to: { opacity: '1' },
        },
        slideUp: {
          from: { opacity: '0', transform: 'translateY(8px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          from: { opacity: '0', transform: 'scale(0.95)' },
          to: { opacity: '1', transform: 'scale(1)' },
        },
      },
    },
  },
  plugins: [],
};
