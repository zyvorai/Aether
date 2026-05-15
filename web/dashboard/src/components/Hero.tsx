interface HeroProps {
  title: string;
  subtitle: string;
}

export default function Hero({ title, subtitle }: HeroProps) {
  return (
    <div className="relative border-b border-slate-800/70">
      {/* Background and glows: overflow clipped here so hero copy is never cropped */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
        <div className="absolute inset-0 bg-gradient-to-r from-slate-950/80 via-slate-950/30 to-slate-950/80" />
        <div className="absolute inset-0 steel-grid opacity-50" />
        <div className="absolute -top-16 left-0 h-64 w-64 rounded-full bg-aether/10 blur-3xl" />
        <div className="absolute top-0 right-0 h-72 w-72 rounded-full bg-cyan-400/10 blur-3xl" />
      </div>

      <div className="relative dash-content py-10 lg:py-12">
        <div className="w-full min-w-0">
          <div className="inline-flex max-w-full flex-wrap items-center gap-x-2 gap-y-1 rounded-full border border-aether/20 bg-aether/10 px-3 py-1.5 text-[11px] font-medium uppercase tracking-[0.18em] text-aether leading-snug">
            Universal runtime control plane
          </div>
          <h1 className="mt-4 text-3xl sm:text-4xl lg:text-5xl font-semibold tracking-tight text-white">{title}</h1>
          <p className="text-slate-400 mt-3 max-w-6xl text-sm sm:text-base leading-7">{subtitle}</p>
        </div>
      </div>
    </div>
  );
}
