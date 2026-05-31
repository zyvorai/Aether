// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

export interface HeroBadge {
  label: string;
  tone: 'brand' | 'ok' | 'warn' | 'info';
}

interface HeroProps {
  title: string;
  subtitle: string;
  badges?: HeroBadge[];
}

const badgeTone: Record<HeroBadge['tone'], string> = {
  brand: 'border-aether/25 bg-aether/10 text-aether',
  ok: 'border-emerald-500/25 bg-emerald-500/10 text-emerald-300',
  warn: 'border-amber-500/25 bg-amber-500/10 text-amber-200',
  info: 'border-sky-500/25 bg-sky-500/10 text-sky-200',
};

export default function Hero({ title, subtitle, badges }: HeroProps) {
  return (
    <div className="relative border-b border-slate-800/50">
      <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
        <div className="absolute inset-0 bg-gradient-to-r from-[#0a0d12]/95 via-[#11151C]/40 to-[#0a0d12]/95" />
        <div className="absolute inset-0 steel-grid opacity-40" />
        <div className="hero-orb absolute -top-20 left-0 h-72 w-72 rounded-full bg-aether/14" />
        <div className="hero-orb hero-orb-delayed absolute -top-10 right-8 h-80 w-80 rounded-full bg-aether-ai/10" />
        <div className="absolute bottom-0 left-1/4 h-px w-1/2 bg-gradient-to-r from-transparent via-aether/35 to-transparent" />
      </div>

      <div className="relative dash-content py-8 lg:py-10">
        <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
          <div className="w-full min-w-0">
            <div className="live-intelligence-badge mb-4 !border-aether/25 !bg-aether/10">
              <span className="live-intelligence-dot" />
              <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-aether">Control Plane</span>
            </div>
            <h1 className="max-w-4xl bg-gradient-to-br from-white via-slate-100 to-slate-400 bg-clip-text text-3xl font-semibold tracking-tight text-transparent sm:text-4xl lg:text-5xl">
              {title}
            </h1>
            {subtitle.trim() ? (
              <p className="mt-3 max-w-2xl text-sm leading-relaxed text-slate-400 sm:text-base">{subtitle}</p>
            ) : null}
          </div>
          {badges && badges.length > 0 ? (
            <div className="flex max-w-xl flex-wrap gap-2 lg:justify-end">
              {badges.map((badge) => (
                <span
                  key={badge.label}
                  className={`inline-flex items-center rounded-full border px-3 py-1.5 text-[11px] font-medium uppercase tracking-[0.14em] shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] ${badgeTone[badge.tone]}`}
                >
                  {badge.label}
                </span>
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
