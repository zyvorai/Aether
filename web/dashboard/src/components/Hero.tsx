interface HeroProps {
  title: string;
  subtitle: string;
}

export default function Hero({ title, subtitle }: HeroProps) {
  return (
    <div className="relative overflow-hidden border-b border-zinc-800">
      {/* Gradient background — subtle zinc with faint aether tint */}
      <div className="absolute inset-0 bg-gradient-to-br from-aether/3 via-zinc-950 to-zinc-950" />
      <div className="absolute top-0 right-0 w-96 h-96 bg-aether/[0.02] rounded-full blur-3xl -translate-y-1/2 translate-x-1/3" />

      <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <h1 className="text-2xl sm:text-3xl font-bold text-aether">{title}</h1>
        <p className="text-zinc-400 mt-1 text-sm sm:text-base">{subtitle}</p>
      </div>
    </div>
  );
}
