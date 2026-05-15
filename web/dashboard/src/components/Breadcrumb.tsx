import { ChevronRight, Home } from 'lucide-react';
import type { AppView } from '../types/api';

const VIEW_LABELS: Record<AppView, string> = {
  overview: 'Dashboard',
  workloads: 'Workloads',
  clusters: 'Cluster Browser',
  compose: 'Compose Import',
  ai: 'AI Engine',
  cost: 'Cost Estimation',
  affinity: 'Runtime Affinity',
  drift: 'Drift Detection',
  policy: 'Policy Check',
  scheduler: 'Scheduler',
  health: 'Health Monitor',
  events: 'Events',
  sla: 'SLA Compliance',
  deps: 'Dependencies',
  envs: 'Environments',
  secrets: 'Secrets',
  backups: 'Backups',
  templates: 'Templates',
  plugins: 'Plugins',
  rbac: 'Access Control',
  audit: 'Audit Trail',
  metrics: 'Metrics',
};

interface BreadcrumbProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}

export default function Breadcrumb({ currentView, onNavigate }: BreadcrumbProps) {
  if (currentView === 'overview') return null;

  const label = VIEW_LABELS[currentView];

  return (
    <nav className="mb-5 flex items-center gap-1.5 text-sm flex-wrap" aria-label="Breadcrumb">
      <button
        type="button"
        onClick={() => onNavigate('overview')}
        className="text-slate-400 hover:text-slate-100 transition flex items-center gap-1 rounded-lg px-1 py-0.5 -mx-1 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/50"
      >
        <Home className="w-3.5 h-3.5 shrink-0" aria-hidden />
        <span className="hidden sm:inline">Dashboard</span>
      </button>
      <ChevronRight className="w-3.5 h-3.5 text-slate-600 shrink-0" aria-hidden />
      <span className="text-slate-100 font-medium">{label}</span>
    </nav>
  );
}
