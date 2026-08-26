// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        canvas: 'var(--canvas)',
        page: 'var(--page)',
        sunken: 'var(--sunken)',
        raised: 'var(--raised)',
        hover: 'var(--hover)',
        selected: 'var(--selected)',
        rule: 'var(--rule)',
        'rule-strong': 'var(--rule-strong)',
        ink: 'var(--ink)',
        'ink-2': 'var(--ink-2)',
        'ink-3': 'var(--ink-3)',
        brand: 'rgb(var(--brand-rgb) / <alpha-value>)',
        'brand-ink': 'var(--brand-ink)',
        'brand-wash': 'var(--brand-wash)',
        danger: 'var(--red)',
        warn: 'var(--amber)',
        success: 'var(--green)',
        scrim: 'var(--scrim)',
        glass: 'var(--glass)',
        deepblue: 'rgb(var(--iphone-deep-blue-rgb) / <alpha-value>)',
        sage: 'rgb(var(--iphone-sage-rgb) / <alpha-value>)',
        mistblue: 'rgb(var(--iphone-mist-blue-rgb) / <alpha-value>)',
        lavender: 'rgb(var(--iphone-lavender-rgb) / <alpha-value>)',
        gold: 'rgb(var(--iphone-gold-rgb) / <alpha-value>)',
        aether: {
          DEFAULT: '#3B82F6',
          light: '#60A5FA',
          dark: '#2563EB',
          dim: 'rgba(59, 130, 246, 0.14)',
          glow: 'rgba(59, 130, 246, 0.28)',
        },
        'aether-ai': {
          DEFAULT: '#A855F7',
          light: '#C084FC',
          dark: '#9333EA',
          dim: 'rgba(168, 85, 247, 0.14)',
          glow: 'rgba(168, 85, 247, 0.28)',
        },
      },
      fontFamily: {
        sans: ['var(--sans)'],
        mono: ['var(--mono)'],
      },
      boxShadow: {
        ambient: 'var(--shadow)',
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'scale-in': 'scaleIn 0.2s ease-out',
        'slide-right': 'slideRight 0.3s ease-out',
        'ac-slide': 'acSlide 0.26s cubic-bezier(0.32, 0.72, 0, 1)',
        'ac-lift': 'acLift 0.2s cubic-bezier(0.32, 0.72, 0, 1)',
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
        slideRight: {
          from: { opacity: '0', transform: 'translateX(100%)' },
          to: { opacity: '1', transform: 'translateX(0)' },
        },
        acSlide: {
          from: { transform: 'translateX(14px)', opacity: '0' },
          to: { transform: 'none', opacity: '1' },
        },
        acLift: {
          from: { transform: 'translateY(8px) scale(0.985)', opacity: '0' },
          to: { transform: 'none', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
};
