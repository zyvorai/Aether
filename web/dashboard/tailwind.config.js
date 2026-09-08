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
        glass: 'var(--glass)',
        deepblue: 'rgb(var(--iphone-deep-blue-rgb) / <alpha-value>)',
        sage: 'rgb(var(--iphone-sage-rgb) / <alpha-value>)',
        mistblue: 'rgb(var(--iphone-mist-blue-rgb) / <alpha-value>)',
        lavender: 'rgb(var(--iphone-lavender-rgb) / <alpha-value>)',
        gold: 'rgb(var(--iphone-gold-rgb) / <alpha-value>)',
        aether: {
          DEFAULT: '#f97316',
          light: '#fb923c',
          dark: '#c2410c',
          dim: 'rgba(249, 115, 22, 0.14)',
          glow: 'rgba(249, 115, 22, 0.28)',
        },
        'aether-ai': {
          DEFAULT: '#f97316',
          light: '#fdba74',
          dark: '#c2410c',
          dim: 'rgba(249, 115, 22, 0.14)',
          glow: 'rgba(249, 115, 22, 0.28)',
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
