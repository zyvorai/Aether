// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router';
import { Bot, CheckCircle2, Loader2, Sparkles, Wrench } from 'lucide-react';
import { apiPost } from '../utils/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import { applyTroubleshootAction,
  isApplyableRecommendation,
  TROUBLESHOOT_ACTION_LABELS,
} from '../utils/troubleshootActions';
import GlassSection from './GlassSection';
import type { DiagnoseResponse, WorkloadResponse } from '../types/api';

interface AiTroubleshootPanelProps {
  workload: WorkloadResponse;
  compact?: boolean;
  onApplied?: () => void;
}

export default function AiTroubleshootPanel({ workload, compact = false, onApplied }: AiTroubleshootPanelProps) {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState<string | null>(null);
  const [report, setReport] = useState<DiagnoseResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appliedMessage, setAppliedMessage] = useState<string | null>(null);

  const run = useCallback(async () => {
    setLoading(true);
    setError(null);
    setAppliedMessage(null);
    const res = await apiPost<DiagnoseResponse>('/copilot/troubleshoot', {
      workload: workload.name,
      cluster: workload.cluster ?? undefined,
      namespace: workload.namespace ?? undefined,
      kind: workload.kind ?? undefined,
      include_copilot_summary: true,
    });
    setLoading(false);
    if (res.success && res.data) {
      setReport(res.data);
    } else {
      setError(res.error ?? 'Diagnosis failed');
    }
  }, [workload]);

  function handleNavigate(action: string) {
    switch (action) {
      case 'open_logs':
        navigate(pathWithQuery(viewToPath('applications'), { workload: workload.name, tab: 'logs' }));
        break;
      case 'fix_image':
        navigate(pathWithQuery(viewToPath('editor'), { workload: workload.name }));
        break;
      case 'check_network':
        navigate(
          pathWithQuery(viewToPath('clusters'), {
            cluster: workload.cluster ?? undefined,
            namespace: workload.namespace ?? undefined,
            tab: 'network',
          }),
        );
        break;
      case 'view_nodes':
        navigate(pathWithQuery(viewToPath('clusters'), { kind: 'Node' }));
        break;
      case 'reconcile_drift':
        navigate(pathWithQuery(viewToPath('drift'), { workload: workload.name }));
        break;
      default:
        navigate(
          pathWithQuery(viewToPath('copilot'), {
            workload: workload.name,
            q: `How do I fix ${workload.name}?`,
          }),
        );
    }
  }

  async function handleApply(action: string) {
    setApplying(action);
    setError(null);
    setAppliedMessage(null);
    const res = await applyTroubleshootAction(workload, action);
    setApplying(null);
    if (res.success) {
      setAppliedMessage(TROUBLESHOOT_ACTION_LABELS[action] ?? 'Fix applied');
      onApplied?.();
      void run();
    } else {
      setError(res.error ?? 'Apply failed');
    }
  }

  return (
    <GlassSection
      accent="purple"
      testId="ai-troubleshoot-panel"
      title="AI Troubleshooting"
      subtitle="Live cluster evidence — health, events, logs, and recommendations."
      icon={<Sparkles className="h-5 w-5 text-aether-ai" />}
      className={compact ? '!p-3 sm:!p-3' : ''}
      actions={
        <button
          type="button"
          onClick={() => void run()}
          disabled={loading}
          className="btn-primary inline-flex items-center gap-1.5 !px-3 !py-1.5 !text-xs disabled:opacity-50"
          data-testid="ai-troubleshoot-diagnose"
        >
          {loading ? <Loader2 size={14} className="animate-spin" /> : <Bot size={14} />}
          {loading ? 'Analyzing…' : 'Diagnose'}
        </button>
      }
    >
      {error && <p className="mt-3 text-xs text-red-300">{error}</p>}
      {appliedMessage && (
        <p className="mt-3 flex items-center gap-1.5 text-xs text-emerald-300">
          <CheckCircle2 size={14} />
          {appliedMessage}
        </p>
      )}

      {report && (
        <div className="mt-4 space-y-3">
          <div className="glass-drawer border-violet-500/20 p-3">
            <div className="flex flex-wrap items-center gap-2 mb-2">
              <span className="text-xs uppercase tracking-wide text-violet-300/80">{report.health_level}</span>
              <span className="text-sm text-slate-200">{report.summary}</span>
            </div>
            {report.evidence.slice(0, 4).map((line) => (
              <p key={line} className="text-xs text-slate-400 mt-1 font-mono truncate" title={line}>
                {line}
              </p>
            ))}
          </div>

          {report.recommendations.length > 0 && (
            <div className="space-y-2">
              {report.recommendations.map((rec) => {
                const applyable = isApplyableRecommendation(rec);
                return (
                  <div
                    key={`${rec.title}-${rec.action}`}
                    className="rounded-lg border border-amber-500/20 bg-amber-950/15 p-3"
                    data-testid={`ai-troubleshoot-rec-${rec.action}`}
                  >
                    <div className="flex items-start gap-2">
                      <Wrench size={14} className="text-amber-400 shrink-0 mt-0.5" />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-amber-100">{rec.title}</p>
                        <p className="text-xs text-amber-200/70 mt-0.5">{rec.summary}</p>
                        <div className="mt-2 flex flex-wrap gap-2">
                          {applyable && (
                            <button
                              type="button"
                              onClick={() => void handleApply(rec.action)}
                              disabled={applying === rec.action}
                              className="inline-flex items-center gap-1 rounded-lg bg-aether/20 px-3 py-1.5 text-xs font-medium text-aether hover:bg-aether/30 disabled:opacity-50"
                              data-testid={`ai-troubleshoot-apply-${rec.action}`}
                            >
                              {applying === rec.action ? (
                                <Loader2 size={12} className="animate-spin" />
                              ) : null}
                              Apply fix
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => handleNavigate(rec.action)}
                            className="text-xs font-medium text-slate-300 hover:text-white hover:underline"
                          >
                            {TROUBLESHOOT_ACTION_LABELS[rec.action] ?? 'View details'} →
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {report.log_excerpt && (
            <pre className="glass-code-block-body max-h-32 text-[10px] text-slate-400 font-mono">
              {report.log_excerpt.slice(-1200)}
            </pre>
          )}
        </div>
      )}
    </GlassSection>
  );
}
