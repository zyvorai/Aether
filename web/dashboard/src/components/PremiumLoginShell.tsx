// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/**
 * Secure Dark Professional login shell — solid split layout + high-contrast form.
 */
import type { ReactNode } from 'react';
import { AlertCircle } from 'lucide-react';

export type LoginOrb = {
  size: number;
  top: string;
  left: string;
  delay: string;
  duration: string;
  hue?: 'blue' | 'violet' | 'cyan' | 'red';
};

export type PremiumLoginFeature = {
  icon: ReactNode;
  title: string;
  description: string;
  tag?: string;
  gradient?: string;
  glow?: string;
  highlight?: boolean;
};

export type PremiumLoginPill = {
  icon?: ReactNode;
  label: string;
  glow?: boolean;
};

export type LoginAccent =
  | 'blue'
  | 'amber'
  | 'orange'
  | 'violet'
  | 'rose'
  | 'cyan'
  | 'copper'
  | 'steel';

function FeatureCard({
  feature: f,
  index: i,
  compact = false,
}: {
  feature: PremiumLoginFeature;
  index: number;
  compact?: boolean;
}) {
  return (
    <div
      className={`login-feature-card login-fade-in flex items-start gap-4 rounded-xl ${
        compact ? 'login-mobile-feature-card' : 'p-4'
      } ${f.highlight ? 'login-feature-card-highlight' : ''}`}
      style={{ animationDelay: `${0.35 + i * 0.07}s` }}
      role="listitem"
    >
      <div
        className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-lg flex items-center justify-center shrink-0 bg-[#11151C] border border-[#1F2937] text-blue-400`}
      >
        {f.icon}
      </div>
      <div className="min-w-0">
        <div className="text-sm font-semibold text-slate-100 flex flex-wrap items-center gap-1.5">
          <span className={compact ? 'text-xs' : undefined}>{f.title}</span>
          {f.tag ? <span className="login-feature-tag">{f.tag}</span> : null}
        </div>
        {!compact ? (
          <p className="text-xs mt-1 text-slate-400 leading-relaxed">{f.description}</p>
        ) : null}
      </div>
    </div>
  );
}

export type PremiumLoginShellProps = {
  accent?: LoginAccent;
  pageThemeClass?: string;
  heroWidth?: '55' | '58';
  themeSwitcher?: ReactNode;
  logo: ReactNode;
  productName: string;
  productSubtitle?: string;
  heroHeadline: ReactNode;
  heroSubheadline: string;
  pills?: PremiumLoginPill[];
  features?: PremiumLoginFeature[];
  mobileFeatures?: PremiumLoginFeature[];
  heroFooter?: ReactNode;
  orbs?: LoginOrb[];
  mobileSubtitle?: string;
  panelTitle?: string;
  panelSubtitle?: string;
  panelHint?: ReactNode;
  footer?: ReactNode;
  formClassName?: string;
  skipToContentLabel?: string;
  showSkipToContent?: boolean;
  children: ReactNode;
};

export function PremiumLoginShell({
  accent = 'blue',
  pageThemeClass = '',
  heroWidth = '58',
  themeSwitcher,
  logo,
  productName,
  productSubtitle,
  heroHeadline,
  heroSubheadline,
  pills = [],
  features = [],
  mobileFeatures,
  heroFooter,
  mobileSubtitle,
  panelTitle = 'Welcome back',
  panelSubtitle = 'Sign in to continue',
  panelHint,
  footer,
  formClassName = '',
  skipToContentLabel = 'Skip to sign in form',
  showSkipToContent = true,
  children,
}: PremiumLoginShellProps) {
  const accentClass = accent === 'blue' ? '' : `login-accent-${accent}`;
  const heroClass = heroWidth === '55' ? 'lg:w-[55%]' : 'lg:w-[58%]';
  const stripFeatures = mobileFeatures ?? features.slice(0, 3);

  return (
    <div className="min-h-screen flex flex-col bg-[#0A0C11]">
      <div
        className={`login-page flex-1 flex flex-col lg:flex-row relative overflow-hidden ${accentClass} ${pageThemeClass}`.trim()}
      >
        {themeSwitcher}

        <aside
          className={`login-hero hidden lg:flex ${heroClass} flex-col p-10 xl:p-12 overflow-hidden relative`}
        >
          <div className="relative z-10 flex flex-col flex-1 min-h-0 gap-6">
            <div>
              <div className="login-fade-in flex items-center gap-4 mb-8">
                <div className="login-logo-ring">{logo}</div>
                <div>
                  <span className="text-3xl font-semibold tracking-tight text-slate-100 block">{productName}</span>
                  {productSubtitle ? (
                    <span className="text-xs font-medium uppercase tracking-[0.2em] text-slate-400 mt-0.5 block">
                      {productSubtitle}
                    </span>
                  ) : null}
                </div>
              </div>
              <h2 className="login-fade-in login-fade-in-d1 text-3xl font-semibold text-slate-100 leading-tight mb-4 max-w-xl">
                {heroHeadline}
              </h2>
              <p className="login-fade-in login-fade-in-d2 text-base text-slate-400 max-w-lg leading-relaxed">
                {heroSubheadline}
              </p>
              {pills.length > 0 ? (
                <div className="login-fade-in login-fade-in-d3 flex flex-wrap gap-2 mt-6">
                  {pills.map((pill) => (
                    <span
                      key={pill.label}
                      className={`login-stat-pill${pill.glow ? ' login-stat-pill-glow' : ''}`}
                    >
                      {pill.icon}
                      {pill.label}
                    </span>
                  ))}
                </div>
              ) : null}
            </div>

            {features.length > 0 ? (
              <ul
                className="relative z-10 login-feature-grid flex-1 min-h-0 list-none m-0 p-0"
                aria-label="Platform capabilities"
                role="list"
              >
                {features.map((f, i) => (
                  <li key={f.title} className="list-none">
                    <FeatureCard feature={f} index={i} />
                  </li>
                ))}
              </ul>
            ) : null}

            {heroFooter ? <div className="relative z-10 login-fade-in login-fade-in-d4 shrink-0">{heroFooter}</div> : null}
          </div>
        </aside>

        {showSkipToContent ? (
          <a href="#login-form" className="login-skip-link">
            {skipToContentLabel}
          </a>
        ) : null}

        <main className="login-panel flex-1 flex items-center justify-center relative px-6 py-12 min-h-screen lg:min-h-0">
          <div className="w-full max-w-[420px] relative z-10">
            <div className="lg:hidden text-center mb-6">
              <div className="login-logo-ring inline-block mb-4">{logo}</div>
              <h1 className="text-2xl font-semibold text-slate-100">{productName}</h1>
              <p className="text-sm mt-1 text-slate-400">{mobileSubtitle ?? productSubtitle ?? panelSubtitle}</p>
            </div>

            {stripFeatures.length > 0 ? (
              <div
                className="login-mobile-features lg:hidden"
                aria-label="Platform capabilities"
                role="list"
              >
                {stripFeatures.map((f, i) => (
                  <div key={f.title} role="listitem">
                    <FeatureCard feature={f} index={i} compact />
                  </div>
                ))}
              </div>
            ) : null}

            <div className="hidden lg:block mb-8">
              <h2 className="text-2xl font-semibold mb-1 text-slate-100">{panelTitle}</h2>
              <p className="text-sm text-slate-400">{panelSubtitle}</p>
            </div>

            <div className={`login-glass login-glass-border p-8 ${formClassName}`.trim()}>
              {children}
              <p className="login-security-footer" role="status">
                RBAC-aware · Encrypted session
              </p>
            </div>

            {panelHint ? (
              <p className="text-xs text-center mt-4 max-w-sm mx-auto leading-relaxed text-slate-500">{panelHint}</p>
            ) : null}
          </div>
        </main>
      </div>
      {footer}
    </div>
  );
}

export function LoginError({ message }: { message: string }) {
  return (
    <div
      className="flex items-center gap-2.5 bg-red-950/50 border border-red-500/40 rounded-lg p-3 mb-6 login-shake"
      role="alert"
      aria-live="polite"
    >
      <AlertCircle className="h-4 w-4 text-red-400 shrink-0" aria-hidden />
      <span className="text-sm text-red-300">{message}</span>
    </div>
  );
}

export function LoginField({
  label,
  id,
  children,
}: {
  label: string;
  id: string;
  children: ReactNode;
}) {
  return (
    <div>
      <label htmlFor={id} className="block text-sm font-medium text-slate-300 mb-2">
        {label}
      </label>
      <div className="relative group">{children}</div>
    </div>
  );
}

export function LoginSubmit({
  loading,
  disabled,
  children,
  className = '',
}: {
  loading?: boolean;
  disabled?: boolean;
  children: ReactNode;
  className?: string;
}) {
  return (
    <button
      type="submit"
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={`login-btn-primary group ${className}`.trim()}
    >
      {children}
    </button>
  );
}

export function LoginRemember({
  checked,
  onChange,
  label = 'Remember me on this device',
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
}) {
  return (
    <label className="flex items-center gap-2.5 mt-5 cursor-pointer select-none">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="w-4 h-4 rounded border-slate-600 bg-[#0A0C11] accent-blue-600"
      />
      <span className="text-sm text-slate-400">{label}</span>
    </label>
  );
}

export function LoginDivider({ label = 'or' }: { label?: string }) {
  return (
    <div className="relative py-3 mt-4 text-center text-xs uppercase tracking-[0.22em] text-slate-500">
      <span className="relative px-2 bg-[#11151C]">{label}</span>
      <div className="absolute inset-x-0 top-1/2 -translate-y-1/2 border-t border-[#1F2937]" />
    </div>
  );
}

/** @deprecated typo guard — use PremiumLoginShell */
export const PremumLoginShell = PremiumLoginShell;
