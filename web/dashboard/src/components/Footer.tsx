import { Hexagon } from 'lucide-react';

export default function Footer() {
  const buildLabel = __AETHER_DASHBOARD_BUILD__.replace('T', ' ').slice(0, 19);

  return (
    <footer className="mt-auto border-t border-slate-800/70 bg-slate-950/80 backdrop-blur-sm">
      <div className="dash-content flex flex-col items-center justify-center gap-2 py-10 text-center sm:flex-row sm:justify-between sm:text-left">
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-aether/20 bg-aether/10">
            <Hexagon className="h-4 w-4 text-aether" aria-hidden />
          </div>
          <div>
            <div className="text-sm font-semibold text-white">Aether</div>
            <div className="text-[11px] font-medium uppercase tracking-[0.22em] text-slate-500">Universal Runtime Control Plane</div>
          </div>
        </div>
        <div className="text-xs text-slate-600">
          <div>&copy; {new Date().getFullYear()} Aether</div>
          <div className="mt-1 font-mono text-[10px] uppercase tracking-[0.16em] text-slate-700">
            UI build {buildLabel}
          </div>
        </div>
      </div>
    </footer>
  );
}
