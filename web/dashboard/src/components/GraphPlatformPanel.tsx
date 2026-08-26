// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import {
  Database,
  Download,
  GitBranch,
  Loader2,
  Radar,
  RefreshCw,
  Search,
  ShieldAlert,
  Target,
} from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge from './Badge';
import type {
  BlastRadiusReport,
  CmdbSyncReport,
  GraphExportReport,
  GraphImpactReport,
  GraphPlacementReport,
  GraphSearchReport,
  GraphSnapshotsReport,
  K8sImportReport,
  ThreatPathsReport,
} from '../types/api';
import GlassSection from './GlassSection';

type Tab = 'impact' | 'blast' | 'threats' | 'search' | 'snapshots' | 'cmdb' | 'placement' | 'export';

export default function GraphPlatformPanel() {
  const [tab, setTab] = useState<Tab>('impact');
  const [loading, setLoading] = useState(true);
  const [workload, setWorkload] = useState('');
  const [searchQ, setSearchQ] = useState('');
  const [impact, setImpact] = useState<GraphImpactReport | null>(null);
  const [blast, setBlast] = useState<BlastRadiusReport | null>(null);
  const [threats, setThreats] = useState<ThreatPathsReport | null>(null);
  const [search, setSearch] = useState<GraphSearchReport | null>(null);
  const [snapshots, setSnapshots] = useState<GraphSnapshotsReport | null>(null);
  const [cmdb, setCmdb] = useState<CmdbSyncReport | null>(null);
  const [placement, setPlacement] = useState<GraphPlacementReport | null>(null);
  const [k8sImport, setK8sImport] = useState<K8sImportReport | null>(null);
  const [exportReport, setExportReport] = useState<GraphExportReport | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [imp, bl, th, sn, cm, pl] = await Promise.all([
      apiFetch<GraphImpactReport>(`/intelligence/graph/impact?workload=${encodeURIComponent(workload)}`),
      apiFetch<BlastRadiusReport>(`/intelligence/graph/blast-radius?workload=${encodeURIComponent(workload)}`),
      apiFetch<ThreatPathsReport>('/intelligence/graph/threat-paths'),
      apiFetch<GraphSnapshotsReport>('/intelligence/graph/snapshots'),
      apiFetch<CmdbSyncReport>('/intelligence/graph/cmdb'),
      apiFetch<GraphPlacementReport>('/intelligence/graph/placement'),
    ]);
    const kiRes = await apiPost<K8sImportReport>('/intelligence/graph/import-k8s', { dry_run: true });
    setImpact(imp);
    setBlast(bl);
    setThreats(th);
    setSnapshots(sn);
    setCmdb(cm);
    setPlacement(pl);
    setK8sImport(kiRes.data ?? null);
    setLoading(false);
  }, [workload]);

  useEffect(() => {
    void load();
  }, [load]);

  async function runSearch() {
    const data = await apiFetch<GraphSearchReport>(`/intelligence/graph/search?q=${encodeURIComponent(searchQ)}`);
    setSearch(data);
  }

  async function captureSnapshot() {
    const res = await apiPost<{ captured: { label: string } }>('/intelligence/graph/snapshots/capture', {
      label: 'Dashboard capture',
    });
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: 'Graph snapshot saved', type: 'success' } }),
      );
      const sn = await apiFetch<GraphSnapshotsReport>('/intelligence/graph/snapshots');
      setSnapshots(sn);
    }
  }

  async function syncCmdb() {
    const res = await apiPost<CmdbSyncReport>('/intelligence/graph/cmdb/sync', { dry_run: true });
    if (res.success && res.data) setCmdb(res.data);
  }

  async function loadExport(format: 'neo4j' | 'jsonld') {
    const data = await apiFetch<GraphExportReport>(`/intelligence/graph/export?format=${format}`);
    setExportReport(data);
  }

  const tabs: { id: Tab; label: string }[] = [
    { id: 'impact', label: 'Impact' },
    { id: 'blast', label: 'Blast radius' },
    { id: 'threats', label: 'Threat paths' },
    { id: 'search', label: 'Graph search' },
    { id: 'snapshots', label: 'Snapshots' },
    { id: 'cmdb', label: 'CMDB' },
    { id: 'placement', label: 'Placement' },
    { id: 'export', label: 'Export' },
  ];

  return (
        <GlassSection
      accent="blue"
      testId="graph-platform-panel"
      title="Graph Intelligence Platform"
      subtitle="Impact, blast radius, CMDB sync, placement, and export"
      icon={<Radar className="h-5 w-5 text-cyan-400" />}
      actions={<div className="flex flex-wrap items-center gap-2">
          <input
            value={workload}
            onChange={(e) => setWorkload(e.target.value)}
            placeholder="workload"
            className="glass-input"
            data-testid="graph-workload-input"
          />
          <button type="button" onClick={() => void load()} className="rounded-xl border glass-divider px-3 py-2 text-xs text-ink-2">
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          </button>
        </div>}
    >{k8sImport ? (
        <p className="mb-4 text-xs text-ink-3" data-testid="k8s-import-status">
          K8s services discovered: {k8sImport.services.length} (dry-run import)
        </p>
      ) : null}

      <div className="mb-6 flex flex-wrap gap-2">
        {tabs.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`rounded-full border px-3 py-1 text-xs ${
              tab === t.id ? 'border-cyan-500/40 bg-cyan-500/10 text-cyan-200' : 'glass-divider text-ink-2'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'impact' ? (
        <div data-testid="graph-impact-panel">
          <p className="mb-3 text-sm text-ink-2">{impact?.summary}</p>
          <ul className="space-y-1">
            {(impact?.downstream ?? []).map((w) => (
              <li key={w} className="text-sm text-ink-2">
                <Target className="mr-1 inline h-3 w-3" />
                {w}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'blast' ? (
        <div data-testid="graph-blast-radius-panel">
          <p className="mb-3 text-lg font-semibold text-amber-300">
            Score {((blast?.blast_score ?? 0) * 100).toFixed(0)}/100
          </p>
          <p className="mb-2 text-sm text-ink-2">{blast?.summary}</p>
          <ul className="space-y-1">
            {(blast?.migration_blockers ?? []).map((b) => (
              <li key={b} className="text-xs text-amber-200">
                {b}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'threats' ? (
        <ul className="space-y-2" data-testid="threat-paths-panel">
          {(threats?.paths ?? []).slice(0, 8).map((p, i) => (
            <li key={i} className="rounded-lg border glass-divider px-3 py-2 text-sm">
              <ShieldAlert className="mr-1 inline h-3.5 w-3.5 text-red-400" />
              {p.summary}
              <div className="mt-1 text-xs text-ink-3">{p.path.join(' → ')}</div>
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'search' ? (
        <div data-testid="graph-search-panel">
          <div className="mb-4 flex gap-2">
            <input
              value={searchQ}
              onChange={(e) => setSearchQ(e.target.value)}
              placeholder="Search nodes…"
              className="glass-input flex-1"
              data-testid="graph-search-input"
            />
            <button
              type="button"
              onClick={() => void runSearch()}
              className="inline-flex items-center gap-2 rounded-lg border border-cyan-500/30 px-3 py-2 text-xs text-cyan-200"
            >
              <Search className="h-3.5 w-3.5" />
              Search
            </button>
          </div>
          <ul className="space-y-2">
            {(search?.hits ?? []).map((h) => (
              <li key={h.id} className="rounded-lg border glass-divider px-3 py-2 text-sm text-ink-2">
                {h.label} <Badge text={h.kind} variant="muted" />
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'snapshots' ? (
        <div data-testid="graph-snapshots-panel">
          <button
            type="button"
            onClick={() => void captureSnapshot()}
            className="mb-4 rounded-xl border border-violet-500/30 bg-violet-500/10 px-3 py-2 text-xs text-violet-200"
          >
            Capture snapshot
          </button>
          <ul className="space-y-2">
            {(snapshots?.snapshots ?? []).map((s) => (
              <li key={s.id} className="text-sm text-ink-2">
                {s.label} · {s.node_count} nodes · {s.captured_at}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'cmdb' ? (
        <div data-testid="cmdb-sync-panel">
          <button
            type="button"
            onClick={() => void syncCmdb()}
            className="mb-4 inline-flex items-center gap-2 rounded-xl border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200"
          >
            <Database className="h-3.5 w-3.5" />
            Sync CMDB (dry-run)
          </button>
          <ul className="space-y-2">
            {(cmdb?.items ?? []).map((item) => (
              <li key={item.id} className="text-sm text-ink-2">
                {item.name} <Badge text={item.item_type} variant="muted" />
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {tab === 'placement' ? (
        <ul className="space-y-2" data-testid="graph-placement-panel">
          {(placement?.entries ?? []).slice(0, 8).map((e) => (
            <li key={e.workload} className="rounded-lg border glass-divider px-3 py-2 text-sm text-ink-2">
              #{e.startup_order} {e.workload}
              {e.recommended_cluster ? <Badge text={e.recommended_cluster} variant="green" /> : null}
              {e.co_locate_with ? (
                <span className="ml-2 text-xs text-ink-3">co-locate with {e.co_locate_with}</span>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}

      {tab === 'export' ? (
        <div data-testid="graph-export-panel">
          <div className="mb-4 flex gap-2">
            <button
              type="button"
              onClick={() => void loadExport('neo4j')}
              className="inline-flex items-center gap-2 rounded-lg border glass-divider px-3 py-2 text-xs text-ink-2"
            >
              <Download className="h-3.5 w-3.5" />
              Neo4j Cypher
            </button>
            <button
              type="button"
              onClick={() => void loadExport('jsonld')}
              className="inline-flex items-center gap-2 rounded-lg border glass-divider px-3 py-2 text-xs text-ink-2"
            >
              <GitBranch className="h-3.5 w-3.5" />
              JSON-LD
            </button>
          </div>
          {exportReport ? (
            <pre className="max-h-48 overflow-auto rounded-xl border glass-divider glass-code-block-body p-3 text-xs text-ink-2">
              {exportReport.payload.slice(0, 2000)}
            </pre>
          ) : null}
        </div>
      ) : null}
    </GlassSection>
  );
}
