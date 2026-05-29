// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { WorkloadResponse } from '../types/api';

export interface ClusterWorkloadRef {
  cluster: string;
  namespace: string;
  kind: string;
  name: string;
}

/** Resolve cluster/namespace/kind/name from a discovered workload row. */
export function parseClusterWorkload(w: WorkloadResponse): ClusterWorkloadRef | null {
  if (w.cluster && w.namespace) {
    const name = w.name.split('/').pop() ?? w.name;
    return {
      cluster: w.cluster,
      namespace: w.namespace,
      kind: w.kind ?? 'Deployment',
      name,
    };
  }
  const parts = w.name.split('/');
  if (parts.length >= 3) {
    return {
      cluster: parts[0],
      namespace: parts[1],
      kind: w.kind ?? 'Deployment',
      name: parts[2],
    };
  }
  return null;
}

export function clusterResourceQuery(ref: ClusterWorkloadRef): string {
  const params = new URLSearchParams({
    cluster: ref.cluster,
    namespace: ref.namespace,
    kind: ref.kind,
    name: ref.name,
  });
  return `/cluster/resource?${params.toString()}`;
}
