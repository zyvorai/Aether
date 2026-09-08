// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from 'react';
import { Box, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge from './Badge';
import type { UnifiedFabricReport } from '../types/api';

export default function UnifiedFabricPanel() {
  const [report, setReport] = useState<UnifiedFabricReport | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<UnifiedFabricReport>('/intelligence/federation/unified-fabric');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const edgeCount = report?.nodes.filter((n) => n.kind === 'edge').length ?? 0;
  const clusterCount = report?.nodes.filter((n) => n.kind === 'cluster').length ?? 0;

  return (
    <section className="glass mb-8 p-6 sm:p-8" data-testid="unified-fabric-panel">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Box className="h-5 w-5 text-aether-ai" />
          <div>
            <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-aether-ai">Fabric</p>
            <h2 className="text-xl font-semibold text-foreground">Unified Edge + Cloud Fabric</h2>
            <p className="text-sm text-muted">Edge agents linked to federated clusters and workloads</p>
          </div>
        </div>
        <button type="button" onClick={() => void load()} className="rounded-xl border glass-divider px-3 py-2 text-xs text-muted transition hover:border-primary/30">
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
        </button>
      </div>
      <div className="mb-4 flex gap-2">
        <Badge text={`${clusterCount} clusters`} variant="muted" />
        <Badge text={`${edgeCount} edge sites`} variant="muted" />
        <Badge text={`${report?.edges.length ?? 0} links`} variant="green" />
      </div>
      <ul className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {(report?.nodes ?? []).slice(0, 12).map((n) => (
          <li key={n.id} className="rounded-xl border glass-divider glass px-3 py-2 text-sm backdrop-blur-sm">
            <div className="flex items-center gap-2">
              <Badge text={n.kind} variant="muted" />
              <span className="font-medium text-foreground">{n.label}</span>
            </div>
            <p className="mt-1 text-xs text-subtle">{n.detail}</p>
          </li>
        ))}
      </ul>
    </section>
  );
}
