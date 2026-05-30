// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import {
  Apple,
  Bell,
  Download,
  Link2,
  Loader2,
  RefreshCw,
  Search,
  Sparkles,
  WifiOff,
} from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import { isMacOSShell } from '../utils/macosBridge';
import Badge from './Badge';
import type {
  DockBadgeReport,
  LiveActivityReport,
  MenuExtrasReport,
  OfflineCacheReport,
  ReleasePipelineReport,
  ShortcutsManifestReport,
  SpotlightIndexReport,
  TraySparklineReport,
  UniversalLinkRegistry,
} from '../types/api';

export default function MacOSPlatformPanel() {
  const [loading, setLoading] = useState(true);
  const [sparkline, setSparkline] = useState<TraySparklineReport | null>(null);
  const [dock, setDock] = useState<DockBadgeReport | null>(null);
  const [live, setLive] = useState<LiveActivityReport | null>(null);
  const [spotlight, setSpotlight] = useState<SpotlightIndexReport | null>(null);
  const [shortcuts, setShortcuts] = useState<ShortcutsManifestReport | null>(null);
  const [menuExtras, setMenuExtras] = useState<MenuExtrasReport | null>(null);
  const [offline, setOffline] = useState<OfflineCacheReport | null>(null);
  const [links, setLinks] = useState<UniversalLinkRegistry | null>(null);
  const [pipeline, setPipeline] = useState<ReleasePipelineReport | null>(null);
  const inShell = isMacOSShell();

  const load = useCallback(async () => {
    setLoading(true);
    const [sp, db, la, sl, sc, me, off, reg, pipe] = await Promise.all([
      apiFetch<TraySparklineReport>('/intelligence/macos/tray-sparkline'),
      apiFetch<DockBadgeReport>('/intelligence/macos/dock-badge'),
      apiFetch<LiveActivityReport>('/intelligence/macos/live-activity'),
      apiFetch<SpotlightIndexReport>('/intelligence/macos/spotlight'),
      apiFetch<ShortcutsManifestReport>('/intelligence/macos/shortcuts'),
      apiFetch<MenuExtrasReport>('/intelligence/macos/menu-extras'),
      apiFetch<OfflineCacheReport>('/intelligence/macos/offline-cache'),
      apiFetch<UniversalLinkRegistry>('/intelligence/macos/universal-links'),
      apiFetch<ReleasePipelineReport>('/intelligence/macos/release-pipeline'),
    ]);
    setSparkline(sp);
    setDock(db);
    setLive(la);
    setSpotlight(sl);
    setShortcuts(sc);
    setMenuExtras(me);
    setOffline(off);
    setLinks(reg);
    setPipeline(pipe);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function refreshOfflineCache() {
    const res = await apiPost<OfflineCacheReport>('/intelligence/macos/offline-cache', {});
    if (res.data) setOffline(res.data);
  }

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="macos-platform-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Apple className="h-5 w-5 text-slate-200" />
          <div>
            <h2 className="text-xl font-semibold text-white">macOS Native OS</h2>
            <p className="text-sm text-slate-400">Tray sparkline, dock badge, Spotlight, offline cache, deep links</p>
          </div>
        </div>
        <div className="flex gap-2">
          <Badge text={inShell ? 'Tauri shell' : 'Web only'} variant={inShell ? 'green' : 'muted'} />
          <button type="button" onClick={() => void load()} className="rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300">
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>

      <div className="mb-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <div className="glass-metric-card" data-testid="tray-sparkline-panel">
          <div className="font-mono text-2xl text-emerald-300">{sparkline?.sparkline ?? '▁▁▁▁'}</div>
          <div className="text-xs text-slate-500">Tray sparkline · {sparkline?.fleet_health_pct.toFixed(0) ?? 0}%</div>
        </div>
        <div className="glass-metric-card" data-testid="dock-badge-panel">
          <div className="text-2xl font-semibold text-white">{dock?.issue_count ?? 0}</div>
          <div className="text-xs text-slate-500">Dock badge issues</div>
        </div>
        <div className="glass-metric-card" data-testid="offline-cache-panel">
          <div className="flex items-center gap-2 text-sm text-slate-300">
            <WifiOff className="h-4 w-4" />
            {offline?.cached ? 'Briefing cached' : 'No offline cache'}
          </div>
          <button
            type="button"
            onClick={() => void refreshOfflineCache()}
            className="mt-2 text-xs text-aether hover:underline"
          >
            Refresh cache
          </button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <div>
          <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-white">
            <Sparkles className="h-4 w-4 text-violet-400" />
            Live Activity (Lab)
          </h3>
          <ul className="space-y-1 text-sm text-slate-400" data-testid="live-activity-panel">
            {(live?.active ?? []).map((a) => (
              <li key={a.workload}>
                {a.workload} · {a.progress_pct}% · {a.detail}
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-white">
            <Search className="h-4 w-4 text-cyan-400" />
            Spotlight index
          </h3>
          <ul className="max-h-32 space-y-1 overflow-auto text-sm text-slate-400" data-testid="spotlight-index-panel">
            {(spotlight?.items ?? []).slice(0, 6).map((item) => (
              <li key={item.id}>
                {item.title} — {item.deep_link}
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-white">
            <Bell className="h-4 w-4 text-amber-400" />
            Shortcuts (Lab)
          </h3>
          <ul className="space-y-1 text-sm text-slate-400">
            {(shortcuts?.shortcuts ?? []).map((s) => (
              <li key={s.name}>
                {s.phrase} → {s.url}
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-white">
            <Link2 className="h-4 w-4 text-blue-400" />
            Universal links
          </h3>
          <ul className="space-y-1 text-sm text-slate-400" data-testid="universal-links-panel">
            {(links?.routes ?? []).slice(0, 6).map((r) => (
              <li key={r.path}>
                aether://{r.path} → /{r.view}
              </li>
            ))}
          </ul>
        </div>
      </div>

      <div className="mt-6">
        <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-white">
          <Download className="h-4 w-4" />
          Release pipeline
        </h3>
        <div className="mb-2 flex gap-2" data-testid="release-pipeline-panel">
          <Badge
            text={pipeline?.notarization_ready ? 'notarization ready' : 'needs secrets'}
            variant={pipeline?.notarization_ready ? 'green' : 'yellow'}
          />
        </div>
        <ul className="space-y-1 text-xs text-slate-500">
          {(pipeline?.steps ?? []).map((s) => (
            <li key={s.id}>
              {s.label}: {s.status}
            </li>
          ))}
        </ul>
      </div>

      <div className="mt-4">
        <h3 className="mb-2 text-sm font-medium text-white">Menu extras (Lab)</h3>
        <div className="flex flex-wrap gap-2">
          {(menuExtras?.toggles ?? []).map((t) => (
            <Badge key={t.id} text={`${t.label}: ${t.enabled ? 'on' : 'off'}`} variant="muted" />
          ))}
        </div>
      </div>
    </section>
  );
}
