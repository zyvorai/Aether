// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { WorkloadResponse } from '../types/api';
import { applicationLabel } from '../utils/k8sUx';

interface ApplicationTopologyProps {
  workload: WorkloadResponse;
}

function Node({ label, sub }: { label: string; sub?: string }) {
  return (
    <div className="glass-panel-card px-4 py-3 text-center min-w-[140px]">
      <div className="text-sm font-medium text-slate-100">{label}</div>
      {sub && <div className="text-xs text-slate-500 mt-1">{sub}</div>}
    </div>
  );
}

function Arrow() {
  return (
    <div className="flex flex-col items-center text-slate-600 py-1">
      <div className="h-6 w-px bg-white/10" />
      <div className="text-xs">↓</div>
    </div>
  );
}

export default function ApplicationTopology({ workload }: ApplicationTopologyProps) {
  const name = applicationLabel(workload);
  const ns = workload.namespace ?? 'default';
  const kind = workload.kind ?? 'Deployment';

  return (
    <div className="flex flex-col items-center py-4" data-testid="application-topology">
      <Node label="Internet / Users" sub="External traffic" />
      <Arrow />
      <Node label={`${name}-ingress`} sub="Ingress / Gateway" />
      <Arrow />
      <Node label={`${name}-service`} sub="ClusterIP / LB" />
      <Arrow />
      <Node label={`${kind}: ${name}`} sub={`Namespace: ${ns}`} />
      <Arrow />
      <Node label="Pods" sub={`Managed by ${kind}`} />
      <Arrow />
      <Node label="Storage / Secrets" sub="PVC · ConfigMap · Secret" />
      <p className="mt-6 text-xs text-slate-500 max-w-md text-center">
        This is the logical application stack. Open Cluster Browser for live Service, Ingress, and Pod objects.
      </p>
    </div>
  );
}
