// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Loader2, RefreshCw, ShieldCheck } from 'lucide-react';
import { apiFetch } from '../utils/api';
import GlassSection from './GlassSection';

interface ProductionOverview {
  feature_count: number;
  features: Array<{ phase: number; name: string }>;
}

interface ProductionScorecard {
  readiness_pct: number;
  production_ready: boolean;
  checks: Array<{ label: string; ok: boolean; severity: string }>;
}

interface AuthPlane {
  api_key_configured: boolean;
  oidc_configured: boolean;
  saml_available: boolean;
}

interface HostedPlane {
  tenant_count: number;
  stripe_configured: boolean;
}

export default function ProductionTrustPanel() {
  const [tab, setTab] = useState<'scorecard' | 'auth' | 'ops'>('scorecard');
  const [loading, setLoading] = useState(true);
  const [overview, setOverview] = useState<ProductionOverview | null>(null);
  const [scorecard, setScorecard] = useState<ProductionScorecard | null>(null);
  const [auth, setAuth] = useState<AuthPlane | null>(null);
  const [hosted, setHosted] = useState<HostedPlane | null>(null);
  const [ciJobs, setCiJobs] = useState(0);

  const load = useCallback(async () => {
    setLoading(true);
    const [ov, sc, ap, hp, ci] = await Promise.all([
      apiFetch<ProductionOverview>('/intelligence/production/overview'),
      apiFetch<ProductionScorecard>('/intelligence/production/scorecard'),
      apiFetch<AuthPlane>('/intelligence/production/auth-plane'),
      apiFetch<HostedPlane>('/intelligence/production/hosted-plane'),
      apiFetch<{ jobs: unknown[] }>('/intelligence/production/ci-smoke-manifest'),
    ]);
    setOverview(ov);
    setScorecard(sc);
    setAuth(ap);
    setHosted(hp);
    setCiJobs(ci?.jobs?.length ?? 0);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <GlassSection
      title="Production & Trust"
      subtitle="Era M — readiness scorecard, auth/HA planes, and CI verify manifests"
      icon={<ShieldCheck className="h-5 w-5 text-emerald-400" />}
      testId="production-trust-panel"
      actions={
        <button type="button" onClick={() => void load()} className="btn-secondary text-xs">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </button>
      }
    >
      <div className="mb-4 flex flex-wrap gap-2">
        {(['scorecard', 'auth', 'ops'] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={tab === t ? 'glass-tab-active tab-chip-active' : 'glass-tab tab-chip'}
          >
            {t === 'scorecard' ? 'Scorecard' : t === 'auth' ? 'Auth' : 'Ops'}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-ink-2">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading production trust…
        </div>
      ) : null}

      {!loading && tab === 'scorecard' && scorecard ? (
        <div data-testid="production-scorecard-panel">
          <p className="text-2xl font-semibold text-ink mb-1">
            {scorecard.readiness_pct.toFixed(0)}% ready
          </p>
          <p className="text-sm text-ink-2 mb-3">
            {scorecard.production_ready ? 'Production ready' : 'Action required'} ·{' '}
            {overview?.feature_count ?? 10} trust features
          </p>
          <ul className="space-y-1 text-sm">
            {scorecard.checks.map((c) => (
              <li key={c.label} className="flex justify-between text-ink-2">
                <span>{c.label}</span>
                <span className={c.ok ? 'text-emerald-400' : 'text-amber-400'}>
                  {c.ok ? 'OK' : c.severity}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {!loading && tab === 'auth' && auth ? (
        <div data-testid="production-auth-panel" className="text-sm text-ink-2 space-y-2">
          <p>API key: {auth.api_key_configured ? 'configured' : 'not set'}</p>
          <p>OIDC: {auth.oidc_configured ? 'configured' : 'not set'}</p>
          <p>SAML: {auth.saml_available ? 'available' : 'not enabled'}</p>
        </div>
      ) : null}

      {!loading && tab === 'ops' && hosted ? (
        <div data-testid="production-ops-panel" className="text-sm text-ink-2 space-y-2">
          <p>Hosted tenants: {hosted.tenant_count}</p>
          <p>Stripe: {hosted.stripe_configured ? 'configured' : 'not set'}</p>
          <p>CI smoke jobs: {ciJobs}</p>
        </div>
      ) : null}
    </GlassSection>
  );
}
