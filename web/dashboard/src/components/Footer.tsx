export default function Footer() {
  const buildLabel = __AETHER_DASHBOARD_BUILD__.replace('T', ' ').slice(0, 19);

  return (
    <footer className="mt-auto border-t border-slate-800/70 bg-slate-950/80 backdrop-blur-sm">
      <div className="dash-content flex flex-col items-center justify-center gap-3 py-8 text-center">
        <a href="https://zyvor.dev" target="_blank" rel="noopener noreferrer" title="Zyvor — zyvor.dev">
          <img src="/zyvor-logo.png" alt="Zyvor" className="h-7 w-auto" />
        </a>
        <p className="text-xs text-slate-500">
          <a href="https://zyvor.dev" target="_blank" rel="noopener noreferrer" className="text-aether hover:underline">
            zyvor.dev
          </a>
          {' · '}© @zyvor 2026 · Aether
        </p>
        <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-slate-700">
          UI build {buildLabel}
        </p>
      </div>
    </footer>
  );
}
