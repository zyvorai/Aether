// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { Box, Container, Cpu, FileText, HardDrive, Info, Server, Star, Terminal } from 'lucide-react';
import Badge, { RuntimeBadge } from './Badge';
import { clusterResourceName, hasClusterLogs, isShellableClusterKind } from '../utils/clusterExec';
import { formatRelativeTime } from '../utils/formatters';
import { isAetherManaged } from '../utils/workloadFilters';
import type { WorkloadResponse } from '../types/api';

function statusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy' || s.includes('deployed')) return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (['stopped', 'exited', 'succeeded'].includes(s)) return 'muted';
  return 'yellow';
}

function statusAccent(status: string): string {
  const s = status.toLowerCase();
  if (s === 'running' || s.includes('deployed')) return 'from-emerald-500/30 via-transparent to-transparent';
  if (s === 'error' || s === 'failed') return 'from-red-500/30 via-transparent to-transparent';
  if (['stopped', 'exited', 'succeeded'].includes(s)) return 'from-slate-500/20 via-transparent to-transparent';
  return 'from-amber-500/25 via-transparent to-transparent';
}

function kindGlyph(kind?: string) {
  const k = (kind ?? '').toLowerCase();
  if (k.includes('virtualmachine') || k.includes('vm')) return Server;
  if (k.includes('stateful')) return HardDrive;
  if (k.includes('daemon') || k.includes('job') || k.includes('cron')) return Cpu;
  if (k.includes('pod') || k.includes('container')) return Container;
  return Box;
}

export interface WorkloadCardProps {
  workload: WorkloadResponse;
  pinned: boolean;
  selected?: boolean;
  compact?: boolean;
  canMutate: boolean;
  index?: number;
  onOpen: (tab?: 'overview' | 'logs', openShell?: boolean) => void;
  onTogglePin: () => void;
}

export default function WorkloadCard({
  workload,
  pinned,
  selected = false,
  compact = false,
  canMutate,
  index = 0,
  onOpen,
  onTogglePin,
}: WorkloadCardProps) {
  const shortName = clusterResourceName(workload.name);
  const discovered = !isAetherManaged(workload);
  const locationLabel = [workload.cluster, workload.namespace].filter(Boolean).join('/');
  const showLogs = !discovered || hasClusterLogs(workload.kind);
  const showShell = discovered ? canMutate && isShellableClusterKind(workload.kind) : canMutate;
  const variant = statusVariant(workload.status);
  const running = variant === 'green';
  const KindIcon = kindGlyph(workload.kind ?? undefined);
  const delayMs = Math.min(index, 12) * 35;

  if (compact) {
    return (
      <button
        type="button"
        onClick={() => onOpen('overview')}
        data-testid={`workload-card-${shortName}`}
        className={`group w-full rounded-xl border px-3 py-2.5 text-left transition ${
          selected
            ? 'border-aether/50 bg-aether/10 shadow-[0_0_0_1px_rgba(211,84,0,0.25)]'
            : 'glass-divider glass-inset-surface hover:border-aether/30 hover:bg-white/[0.03]'
        }`}
      >
        <div className="flex items-start gap-2">
          <span
            className={`mt-1 h-2 w-2 shrink-0 rounded-full ${
              variant === 'green' ? 'bg-emerald-400' :
              variant === 'red' ? 'bg-red-400' :
              variant === 'muted' ? 'bg-slate-500' : 'bg-amber-400'
            } ${running ? 'platform-pulse' : ''}`}
          />
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-medium text-slate-100">{shortName}</div>
            <div className="mt-0.5 truncate text-[11px] text-slate-500">
              {[workload.kind, locationLabel || null].filter(Boolean).join(' · ') || workload.runtime}
            </div>
          </div>
          {pinned ? <Star size={12} className="mt-0.5 shrink-0 fill-current text-aether" /> : null}
        </div>
      </button>
    );
  }

  return (
    <article
      data-testid={`workload-card-${shortName}`}
      style={{ animationDelay: `${delayMs}ms` }}
      className={`workload-card-rise group relative overflow-hidden rounded-2xl border transition duration-300 hover:-translate-y-0.5 ${
        selected
          ? 'border-aether/45 bg-gradient-to-br from-aether/15 via-white/[0.03] to-transparent shadow-[0_20px_50px_rgba(0,0,0,0.35)]'
          : 'glass-divider bg-gradient-to-br from-white/[0.05] via-white/[0.015] to-transparent hover:border-aether/35 hover:shadow-[0_18px_44px_rgba(0,0,0,0.32)]'
      }`}
    >
      <div className={`pointer-events-none absolute inset-0 bg-gradient-to-br ${statusAccent(workload.status)}`} aria-hidden />
      <div className="pointer-events-none absolute -right-10 -top-12 h-32 w-32 rounded-full bg-aether/12 blur-3xl transition duration-500 group-hover:bg-aether/22 group-hover:scale-110" aria-hidden />
      <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-white/20 to-transparent opacity-60" aria-hidden />

      <div className="relative flex h-full flex-col p-4">
        <div className="mb-3 flex items-start gap-3">
          <div className="relative mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border glass-divider bg-gradient-to-br from-white/[0.08] to-transparent">
            <KindIcon size={18} className="text-slate-300 transition group-hover:text-aether" />
            {running ? (
              <span className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-[rgba(10,13,18,0.9)] bg-emerald-400 platform-pulse" />
            ) : null}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-start gap-2">
              <button type="button" onClick={() => onOpen('overview')} className="min-w-0 flex-1 text-left">
                <h3 className="truncate text-base font-semibold tracking-tight text-white transition group-hover:text-aether">
                  {shortName}
                </h3>
                <p className="mt-0.5 truncate text-[11px] text-slate-500" title={workload.name}>
                  {[workload.kind, locationLabel || null].filter(Boolean).join(' · ') || 'Aether managed'}
                </p>
              </button>
              <button
                type="button"
                onClick={onTogglePin}
                className={`mt-0.5 shrink-0 transition ${pinned ? 'text-aether' : 'text-slate-600 hover:text-slate-300'}`}
                title={pinned ? 'Unpin' : 'Pin'}
                aria-label={pinned ? `Unpin ${shortName}` : `Pin ${shortName}`}
              >
                <Star size={14} className={pinned ? 'fill-current' : ''} />
              </button>
            </div>
            <div className="mt-2">
              <Badge text={workload.status} variant={variant} />
            </div>
          </div>
        </div>

        <button type="button" onClick={() => onOpen('overview')} className="mb-4 min-w-0 flex-1 text-left">
          <div className="mb-2 flex flex-wrap items-center gap-1.5">
            <RuntimeBadge runtime={workload.runtime} />
            <span className={`rounded-md border px-1.5 py-0.5 text-[10px] uppercase tracking-wider ${
              discovered
                ? 'border-sky-500/25 bg-sky-500/10 text-sky-300'
                : 'border-violet-500/25 bg-violet-500/10 text-violet-200'
            }`}>
              {discovered ? 'discovered' : 'managed'}
            </span>
          </div>
          <code className="block truncate rounded-lg glass-inset-surface px-2.5 py-1.5 font-mono text-[11px] text-slate-300" title={workload.image}>
            {workload.image || '—'}
          </code>
          <div className="mt-2 text-[11px] text-slate-500">
            {workload.created_at ? formatRelativeTime(workload.created_at) : '—'}
          </div>
        </button>

        <div className="flex items-center gap-1 rounded-xl border glass-divider glass-inset-surface p-1">
          <button
            type="button"
            onClick={() => onOpen('overview')}
            className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-white/5 hover:text-white"
            title="Info"
            data-testid={`workload-card-info-${shortName}`}
          >
            <Info size={13} />
            Info
          </button>
          {showLogs ? (
            <button
              type="button"
              onClick={() => onOpen('logs')}
              className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-sky-500/15 hover:text-sky-200"
              title="Logs"
              data-testid={`workload-card-logs-${shortName}`}
            >
              <FileText size={13} />
              Logs
            </button>
          ) : null}
          {showShell ? (
            <button
              type="button"
              onClick={() => onOpen('overview', true)}
              className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-slate-300 transition hover:bg-emerald-500/15 hover:text-emerald-200"
              title="Shell / exec"
              data-testid={`workload-card-shell-${shortName}`}
            >
              <Terminal size={13} />
              Exec
            </button>
          ) : null}
        </div>
      </div>
    </article>
  );
}
