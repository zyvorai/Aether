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
    <div className="relative border-b border-slate-800/70">
      {/* Background and glows: overflow clipped here so hero copy is never cropped */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
        <div className="absolute inset-0 bg-gradient-to-r from-slate-950/80 via-slate-950/30 to-slate-950/80" />
        <div className="absolute inset-0 steel-grid opacity-50" />
        <div className="absolute -top-16 left-0 h-64 w-64 rounded-full bg-aether/10 blur-3xl" />
        <div className="absolute top-0 right-0 h-72 w-72 rounded-full bg-cyan-400/10 blur-3xl" />
      </div>

      <div className="relative dash-content py-8 lg:py-10">
        <div className="w-full min-w-0">
          {badges && badges.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {badges.map((badge) => (
                <span
                  key={badge.label}
                  className={`inline-flex items-center rounded-full border px-3 py-1 text-[11px] font-medium uppercase tracking-[0.14em] ${badgeTone[badge.tone]}`}
                >
                  {badge.label}
                </span>
              ))}
            </div>
          ) : null}
          <h1 className={`text-3xl sm:text-4xl lg:text-5xl font-semibold tracking-tight text-white ${badges && badges.length > 0 ? 'mt-4' : ''}`}>
            {title}
          </h1>
          {subtitle.trim() ? (
            <p className="text-slate-500 mt-2 max-w-2xl text-sm leading-relaxed">{subtitle}</p>
          ) : null}
        </div>
      </div>
    </div>
  );
}
