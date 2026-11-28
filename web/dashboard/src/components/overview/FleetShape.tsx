// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { WorkloadResponse } from '../../types/api';
import { workloadCondition, type WorkloadCondition } from '../../utils/workloadStatus';
import ShareBar, { type ShareSegment } from '../ui/ShareBar';
import type { Tone } from '../layout/PageHero';

const CONDITIONS: { id: WorkloadCondition; label: string; tone: Tone }[] = [
  { id: 'running', label: 'Running', tone: 'emerald' },
  { id: 'degraded', label: 'Degraded', tone: 'amber' },
  { id: 'failing', label: 'Failing', tone: 'pink' },
  { id: 'stopped', label: 'Stopped', tone: 'slate' },
  { id: 'unknown', label: 'Unknown', tone: 'violet' },
];

const RUNTIME_TONES: Record<string, { label: string; tone: Tone }> = {
  podman: { label: 'Podman', tone: 'sky' },
  docker: { label: 'Docker', tone: 'amber' },
  kubernetes: { label: 'Kubernetes', tone: 'violet' },
  kubevirt: { label: 'KubeVirt', tone: 'teal' },
};

/** How the listed workloads divide by state and by runtime. */
export default function FleetShape({
  workloads,
  onRuntime,
}: {
  /** null when the workload list could not be loaded. */
  workloads: WorkloadResponse[] | null;
  onRuntime: (runtime: string) => void;
}) {
  if (workloads === null) {
    return (
      <section className="glass p-5 text-sm text-muted" aria-label="Fleet shape" data-testid="overview-fleet-shape">
        The workload list could not be loaded, so there is nothing to chart.
      </section>
    );
  }

  const byCondition: ShareSegment[] = CONDITIONS.map((c) => ({
    ...c,
    count: workloads.filter((w) => workloadCondition(w.status) === c.id).length,
  }));

  const runtimes = new Map<string, number>();
  for (const w of workloads) {
    const key = w.runtime.toLowerCase();
    runtimes.set(key, (runtimes.get(key) ?? 0) + 1);
  }
  const byRuntime: ShareSegment[] = [...runtimes.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([runtime, count]) => ({
      id: runtime,
      label: RUNTIME_TONES[runtime]?.label ?? (runtime || 'Unreported'),
      tone: RUNTIME_TONES[runtime]?.tone ?? 'slate',
      count,
      onSelect: runtime ? () => onRuntime(runtime) : undefined,
    }));

  return (
    <section className="glass grid gap-x-10 gap-y-5 p-5 md:grid-cols-2" aria-label="Fleet shape" data-testid="overview-fleet-shape">
      <ShareBar title="Workloads by state" segments={byCondition} empty="No workloads listed yet." />
      <ShareBar title="Workloads by runtime" segments={byRuntime} empty="No workloads listed yet." />
    </section>
  );
}
