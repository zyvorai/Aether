// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { AlertTriangle, Boxes, Layers, Rocket, Server, Shield } from 'lucide-react';
import type { AppView, ClusterSummary, WorkloadResponse } from '../types/api';
import { isK8sApplication, healthTone } from '../utils/k8sUx';
import StatCard from './StatCard';

interface K8sControlCenterProps {
  username: string;
  workloads: WorkloadResponse[];
  clusterSummary: ClusterSummary | null;
  onNavigate: (view: AppView) => void;
}

export default function K8sControlCenter({
  username,
  workloads,
  clusterSummary,
  onNavigate,
}: K8sControlCenterProps) {
  const apps = workloads.filter(isK8sApplication);
  const healthy = apps.filter((a) => healthTone(a.status) === 'healthy').length;
  const failing = apps.filter((a) => healthTone(a.status) === 'critical').length;
  const warnings = apps.filter((a) => healthTone(a.status) === 'warning').length;
  const clusters = clusterSummary?.cluster_count ?? 0;
  const greeting = username ? username.split('@')[0] : 'Operator';

  const actions: { label: string; view: AppView; icon: typeof Rocket }[] = [
    { label: 'Deploy App', view: 'editor', icon: Rocket },
    { label: 'Applications', view: 'applications', icon: Boxes },
    { label: 'Helm Store', view: 'helm', icon: Layers },
    { label: 'Activity', view: 'activity', icon: Server },
    { label: 'View Alerts', view: 'alerts', icon: AlertTriangle },
    { label: 'Security', view: 'security', icon: Shield },
  ];

  return (
    <section className="dash-card mb-6" data-testid="k8s-control-center">
      <div className="mb-6">
        <p className="text-sm text-slate-400">Kubernetes Control Center</p>
        <h2 className="text-2xl font-semibold text-slate-100 mt-1">
          Good {new Date().getHours() < 12 ? 'morning' : new Date().getHours() < 17 ? 'afternoon' : 'evening'},{' '}
          {greeting}.
        </h2>
        <p className="text-sm text-slate-400 mt-2">
          {failing === 0 && warnings === 0
            ? 'Production Kubernetes looks healthy.'
            : `${failing + warnings} application${failing + warnings === 1 ? '' : 's'} need attention.`}
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
        <StatCard title="Clusters" value={clusters} color="blue" icon={<Server size={18} />} />
        <StatCard title="Applications" value={apps.length} color="orange" icon={<Boxes size={18} />} />
        <StatCard title="Healthy" value={healthy} color="green" icon={<Shield size={18} />} />
        <StatCard title="Needs fix" value={failing + warnings} color="yellow" icon={<AlertTriangle size={18} />} />
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
        {actions.map(({ label, view, icon: Icon }) => (
          <button
            key={label}
            type="button"
            onClick={() => onNavigate(view)}
            className="flex flex-col items-center gap-2 rounded-xl border border-slate-800 bg-slate-950/50 p-4 text-sm text-slate-200 hover:border-aether/40 hover:bg-aether/5 transition-colors"
          >
            <Icon size={22} className="text-aether" />
            <span className="text-center text-xs font-medium">{label}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
