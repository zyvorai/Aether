// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Apple, Loader2, RefreshCw, Zap } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import GlassSection from './GlassSection';

interface ExtensionsOverview {
  graduated_count: number;
  chaos_live_enabled: boolean;
  features: Array<{ phase: number; name: string; status: string }>;
}

interface ShipChaos {
  status: string;
  live_enabled: boolean;
  catalog: { experiments: Array<{ id: string; label: string; risk: string }> };
}

interface ShipGameDays {
  status: string;
  plan: { scenarios: Array<{ id: string; title: string; duration_minutes: number }> };
}

interface NativeBundle {
  spotlight_items: number;
  shortcuts_count: number;
}

export default function ExtensionsGraduationPanel() {
  const [tab, setTab] = useState<'overview' | 'chaos' | 'native'>('overview');
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<ExtensionsOverview | null>(null);
  const [chaos, setChaos] = useState<ShipChaos | null>(null);
  const [gameDays, setGameDays] = useState<ShipGameDays | null>(null);
  const [native, setNative] = useState<NativeBundle | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    const [ov, ch, gd, nb] = await Promise.all([
      apiFetch<ExtensionsOverview>('/intelligence/extensions/overview'),
      apiFetch<ShipChaos>('/intelligence/extensions/chaos/experiments'),
      apiFetch<ShipGameDays>('/intelligence/extensions/game-days'),
      apiFetch<NativeBundle>('/intelligence/extensions/native-bundle'),
    ]);
    setOverview(ov);
    setChaos(ch);
    setGameDays(gd);
    setNative(nb);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function runChaos(id: string) {
    await apiPost('/intelligence/extensions/chaos/run', { dry_run: true, experiment_id: id });
  }

  async function runGameDay(id: string) {
    await apiPost('/intelligence/extensions/game-days/execute', { dry_run: true, scenario_id: id });
  }

  return (
    <GlassSection
      title="Extensions Graduation"
      subtitle="Era L — SRE chaos, game days, and macOS native features shipped"
      icon={<Zap className="h-5 w-5 text-amber-400" />}
      testId="extensions-graduation-panel"
      actions={
        <button type="button" onClick={() => void load()} className="btn-secondary text-xs">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {(['overview', 'chaos', 'native'] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={tab === t ? 'glass-tab-active tab-chip-active' : 'glass-tab tab-chip'}
          >
            {t === 'overview' ? 'Overview' : t === 'chaos' ? 'SRE Chaos' : 'macOS Native'}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-ink-2">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading extensions…
        </div>
      ) : null}

      {!loading && tab === 'overview' && overview ? (
        <div data-testid="extensions-overview-panel">
          <p className="text-sm text-ink-2 mb-3">
            {overview.graduated_count} features · chaos live{' '}
            {overview.chaos_live_enabled ? 'enabled' : 'dry-run only'}
          </p>
          <ul className="space-y-2 text-sm">
            {overview.features.slice(0, 6).map((f) => (
              <li key={f.phase} className="flex justify-between rounded-xl border glass-divider glass px-3 py-2">
                <span className="text-ink">Phase {f.phase}: {f.name}</span>
                <span className="text-xs text-emerald-400 uppercase">{f.status}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {!loading && tab === 'chaos' && chaos ? (
        <div data-testid="extensions-chaos-panel" className="space-y-4">
          <ul className="space-y-2 text-sm">
            {(chaos.catalog.experiments ?? []).slice(0, 4).map((e) => (
              <li key={e.id} className="flex items-center justify-between rounded-xl border glass-divider px-3 py-2">
                <span className="text-ink">{e.label}</span>
                <button type="button" onClick={() => void runChaos(e.id)} className="btn-secondary text-xs">
                  Dry-run
                </button>
              </li>
            ))}
          </ul>
          {gameDays?.plan.scenarios?.[0] ? (
            <button
              type="button"
              onClick={() => void runGameDay(gameDays.plan.scenarios[0].id)}
              className="btn-primary text-xs"
            >
              Dry-run game day: {gameDays.plan.scenarios[0].title}
            </button>
          ) : null}
        </div>
      ) : null}

      {!loading && tab === 'native' && native ? (
        <div data-testid="extensions-native-panel" className="flex items-center gap-3 text-sm text-ink-2">
          <Apple className="h-5 w-5 text-ink-2" />
          Spotlight {native.spotlight_items} items · {native.shortcuts_count} Shortcuts
        </div>
      ) : null}
    </GlassSection>
  );
}
