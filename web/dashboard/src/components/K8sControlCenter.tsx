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
    <section className="overview-section-shell mb-8 p-6 sm:p-8" data-testid="k8s-control-center">
      <div className="overview-section-header mb-6">
        <p className="section-label">Kubernetes</p>
        <h2 className="section-title">
          Good {new Date().getHours() < 12 ? 'morning' : new Date().getHours() < 17 ? 'afternoon' : 'evening'},{' '}
          {greeting}
        </h2>
        <p className="section-subtitle">
          {failing === 0 && warnings === 0
            ? 'Production Kubernetes looks healthy.'
            : `${failing + warnings} application${failing + warnings === 1 ? '' : 's'} need attention.`}
        </p>
      </div>

      <div className="mb-6 grid grid-cols-2 gap-3 md:grid-cols-4">
        <StatCard title="Clusters" value={clusters} color="blue" icon={<Server size={16} />} compact isEmpty={clusters === 0} />
        <StatCard title="Applications" value={apps.length} color="orange" icon={<Boxes size={16} />} compact isEmpty={apps.length === 0} />
        <StatCard title="Healthy" value={healthy} color="green" icon={<Shield size={16} />} compact isEmpty={healthy === 0} />
        <StatCard title="Needs fix" value={failing + warnings} color="yellow" icon={<AlertTriangle size={16} />} compact isEmpty={failing + warnings === 0} />
      </div>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
        {actions.map(({ label, view, icon: Icon }) => (
          <button
            key={label}
            type="button"
            onClick={() => onNavigate(view)}
            className="quick-link-chip flex flex-col items-center gap-2 py-4"
          >
            <Icon size={20} className="text-brand" />
            <span className="text-center text-xs font-medium">{label}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
