// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Copy, Loader2, RefreshCw, Shield } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';
import Badge, { SeverityBadge } from './Badge';
import type { SecurityCopilotReport, SecurityRemediateReport } from '../types/api';
import GlassSection from './GlassSection';

export default function SecurityCopilotPanel() {
  const [report, setReport] = useState<SecurityCopilotReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState<string | null>(null);
  const [remediating, setRemediating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<SecurityCopilotReport>('/intelligence/security/policies');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function copyYaml(workload: string, yaml: string) {
    try {
      await navigator.clipboard.writeText(yaml);
      setCopied(workload);
      window.setTimeout(() => setCopied(null), 2000);
    } catch {
      window.dispatchEvent(
        new CustomEvent('aether-toast', { detail: { message: 'Copy failed', type: 'error' } }),
      );
    }
  }

  async function remediate(dryRun: boolean) {
    const confirm = dryRun ? false : window.confirm('Apply security remediation policies? High-severity items need confirm.');
    if (!dryRun && !confirm) return;
    setRemediating(true);
    const res = await apiPost<SecurityRemediateReport>('/intelligence/security/remediate', {
      dry_run: dryRun,
      confirm: !dryRun,
    });
    setRemediating(false);
    if (res.success) {
      window.dispatchEvent(
        new CustomEvent('aether-toast', {
          detail: {
            message: dryRun
              ? `Security dry-run: ${res.data?.applied.length ?? 0} policies`
              : `Security remediated ${res.data?.applied.length ?? 0} policies`,
            type: 'success',
          },
        }),
      );
    }
  }

  return (
        <GlassSection
      accent="purple"
      testId="security-copilot-panel"
      title="Security Copilot"
      subtitle="Least-privilege policy drafts generated from live threat intelligence."
      icon={<Shield className="h-5 w-5 text-red-400" />}
      actions={<div className="flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => void remediate(true)}
            disabled={remediating || !(report?.suggestions.length ?? 0)}
            className="rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200 hover:border-red-400/50 disabled:opacity-60"
            data-testid="security-remediate-dry-run"
          >
            Dry-run remediate
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border glass-divider px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>}
    >{!report?.suggestions.length ? (
        <p className="text-sm text-slate-500">No policy suggestions — threat scan is clean.</p>
      ) : (
        <ul className="space-y-4">
          {report.suggestions.map((item) => (
            <li
              key={`${item.workload}-${item.title}`}
              className="glass-panel-card p-4"
            >
              <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-white">{item.title}</span>
                  <Badge text={item.workload} variant="muted" />
                  <SeverityBadge severity={item.severity} />
                </div>
                <button
                  type="button"
                  onClick={() => void copyYaml(item.workload, item.policy_yaml)}
                  className="inline-flex items-center gap-1 rounded-lg border glass-divider px-2 py-1 text-xs text-slate-300 hover:border-aether/40"
                >
                  <Copy className="h-3 w-3" />
                  {copied === item.workload ? 'Copied' : 'Copy YAML'}
                </button>
              </div>
              <p className="mb-3 text-sm text-slate-400">{item.rationale}</p>
              <pre className="max-h-48 overflow-auto rounded-xl border glass-divider glass-code-block-body p-3 text-xs text-slate-300">
                {item.policy_yaml}
              </pre>
            </li>
          ))}
        </ul>
      )}
    </GlassSection>
  );
}
