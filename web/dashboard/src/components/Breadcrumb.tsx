import { ChevronRight, Home } from 'lucide-react';
import type { AppView } from '../types/api';
import { VIEW_LABELS } from '../utils/dashboardNav';

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}

export default function Breadcrumb({ currentView, onNavigate }: BreadcrumbProps) {
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];

  return (
    <nav className="dash-breadcrumb mb-6" aria-label="Breadcrumb">
      <div className="flex min-w-0 flex-wrap items-center gap-1 px-1 py-1">
        <button
          type="button"
          onClick={() => onNavigate('overview')}
          className="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-sm text-slate-400 transition hover:bg-white/[0.04] hover:text-slate-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
        >
          <Home className="h-3.5 w-3.5 shrink-0 opacity-70" aria-hidden />
          <span className="hidden sm:inline">Home</span>
        </button>
        <ChevronRight className="h-3.5 w-3.5 shrink-0 text-slate-600" aria-hidden />
        <span className="truncate px-1 py-1.5 text-sm font-medium text-slate-100">{label}</span>
      </div>
    </nav>
  );
}
