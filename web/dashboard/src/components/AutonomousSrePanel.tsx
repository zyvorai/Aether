// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import { Copy, FileText, Loader2, RefreshCw } from 'lucide-react';
import { apiFetch } from '../utils/api';
import Badge, { SeverityBadge } from './Badge';
import type { SreRunbookReport } from '../types/api';

export default function AutonomousSrePanel() {
  const [report, setReport] = useState<SreRunbookReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await apiFetch<SreRunbookReport>('/intelligence/sre/runbook');
    setReport(data);
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function copyRunbook() {
    if (!report) return;
    try {
      await navigator.clipboard.writeText(report.runbook_markdown);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message: 'Copy failed', type: 'error' } }));
    }
  }

  return (
    <section className="surface-panel rounded-[28px] p-6 sm:p-8" data-testid="autonomous-sre-panel">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <FileText className="h-5 w-5 text-amber-400" />
          <div>
            <h2 className="text-xl font-semibold text-white">Autonomous SRE Runbook</h2>
            <p className="text-sm text-slate-400">
              AI-generated operational brief from fleet health, healing, remediation, and FinOps.
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => void copyRunbook()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            <Copy className="h-3.5 w-3.5" />
            {copied ? 'Copied' : 'Copy markdown'}
          </button>
          <button
            type="button"
            onClick={() => void load()}
            className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-2 text-xs text-slate-300 hover:border-aether/40"
          >
            {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
            Refresh
          </button>
        </div>
      </div>

      {report ? (
        <>
          <p className="mb-6 rounded-2xl border border-amber-500/20 bg-amber-500/5 px-4 py-3 text-sm text-amber-100">
            {report.summary}
          </p>
          <div className="grid gap-4 lg:grid-cols-2">
            {report.sections.map((section) => (
              <div key={section.title} className="rounded-2xl border border-slate-800/70 bg-slate-950/40 p-4">
                <div className="mb-3 flex items-center gap-2">
                  <span className="font-medium text-white">{section.title}</span>
                  <SeverityBadge severity={section.severity} />
                </div>
                <ul className="space-y-2">
                  {section.items.map((item) => (
                    <li key={item} className="text-sm text-slate-400">
                      {item}
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
          <details className="mt-6">
            <summary className="cursor-pointer text-sm text-aether">View full markdown</summary>
            <pre className="mt-3 max-h-64 overflow-auto rounded-xl border border-slate-800 bg-black/40 p-3 text-xs text-slate-300">
              {report.runbook_markdown}
            </pre>
          </details>
        </>
      ) : loading ? (
        <div className="flex items-center gap-2 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          Generating runbook…
        </div>
      ) : null}
    </section>
  );
}
