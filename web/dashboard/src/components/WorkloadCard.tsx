// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { Box, Container, Cpu, FileText, HardDrive, Info, Server, Star, Terminal } from 'lucide-react';
import Badge, { RuntimeBadge } from './Badge';
import { clusterResourceName, hasClusterLogs, isShellableClusterKind } from '../utils/clusterExec';
import { formatRelativeTime } from '../utils/formatters';
import { isAetherManaged } from '../utils/workloadFilters';
import { statusVariant } from '../utils/workloadStatus';
import type { WorkloadResponse } from '../types/api';

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
  const shellable = discovered ? isShellableClusterKind(workload.kind) : true;
  const showShell = shellable && canMutate;
  const showShellDenied = shellable && !canMutate;
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
            ? 'border-primary/50 bg-primary/10 ring-1 ring-primary'
            : 'border-border bg-surface-elevated hover:bg-hover'
        }`}
      >
        <div className="flex items-start gap-2">
          <span
            className={`mt-1 h-2 w-2 shrink-0 rounded-full ${
              variant === 'green' ? 'bg-success' :
              variant === 'red' ? 'bg-danger' :
              variant === 'muted' ? 'bg-subtle' : 'bg-warning'
            } ${running ? 'platform-pulse' : ''}`}
          />
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-medium text-foreground">{shortName}</div>
            <div className="mt-0.5 truncate text-[11px] text-subtle">
              {[workload.kind, locationLabel || null].filter(Boolean).join(' · ') || workload.runtime}
            </div>
          </div>
          {pinned ? <Star size={12} className="mt-0.5 shrink-0 fill-current text-primary" /> : null}
        </div>
      </button>
    );
  }

  return (
    <article
      data-testid={`workload-card-${shortName}`}
      style={{ animationDelay: `${delayMs}ms` }}
      className={`card-rise group relative overflow-hidden rounded-[var(--radius-card)] border transition duration-300 hover:-translate-y-0.5 hover:shadow-[var(--shadow-raised)] ${
        selected
          ? 'border-primary bg-surface-elevated shadow-card'
          : 'border-[var(--card-edge)] bg-surface-elevated shadow-card hover:border-rule-strong'
      }`}
    >
      <div className="relative flex h-full flex-col p-4">
        <div className="mb-3 flex items-start gap-3">
          <div className="relative mt-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border glass-divider bg-surface">
            <KindIcon size={18} className="text-muted transition group-hover:text-primary" />
            {running ? (
              <span className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-surface-elevated bg-success platform-pulse" />
            ) : null}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-start gap-2">
              <button type="button" onClick={() => onOpen('overview')} className="min-w-0 flex-1 text-left">
                <h3 className="truncate text-base font-semibold tracking-tight text-foreground transition group-hover:text-primary">
                  {shortName}
                </h3>
                <p className="mt-0.5 truncate text-[11px] text-subtle" title={workload.name}>
                  {[workload.kind, locationLabel || null].filter(Boolean).join(' · ') || 'Aether managed'}
                </p>
              </button>
              <button
                type="button"
                onClick={onTogglePin}
                className={`mt-0.5 shrink-0 transition ${pinned ? 'text-primary' : 'text-subtle hover:text-muted'}`}
                title={pinned ? 'Unpin' : 'Pin'}
                aria-label={pinned ? `Unpin ${shortName}` : `Pin ${shortName}`}
              >
                <Star size={14} className={pinned ? 'fill-current' : ''} />
              </button>
            </div>
            <div className="mt-2">
              <span title={workload.status_detail ?? undefined}>
                <Badge text={workload.status} variant={variant} />
              </span>
            </div>
          </div>
        </div>

        <button type="button" onClick={() => onOpen('overview')} className="mb-4 min-w-0 flex-1 text-left">
          <div className="mb-2 flex flex-wrap items-center gap-1.5">
            <RuntimeBadge runtime={workload.runtime} />
            <span className={`rounded-md border px-1.5 py-0.5 text-[10px] uppercase tracking-wider ${
              discovered
                ? 'border-primary/25 bg-primary/10 text-primary'
                : 'border-lavender/25 bg-lavender/10 text-lavender'
            }`}>
              {discovered ? 'discovered' : 'managed'}
            </span>
          </div>
          <code className="block truncate rounded-lg glass-inset-surface px-2.5 py-1.5 font-mono text-[11px] text-muted" title={workload.image}>
            {workload.image || '—'}
          </code>
          <div className="mt-2 text-[11px] text-subtle">
            {workload.created_at ? formatRelativeTime(workload.created_at) : '—'}
          </div>
        </button>

        <div className="flex items-center gap-1 rounded-xl border glass-divider glass-inset-surface p-1">
          <button
            type="button"
            onClick={() => onOpen('overview')}
            className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-hover hover:text-foreground"
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
              className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-primary/10 hover:text-primary"
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
              className="inline-flex flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-muted transition hover:bg-success/10 hover:text-success"
              title="Exec into pod"
              data-testid={`workload-card-shell-${shortName}`}
            >
              <Terminal size={13} />
              Exec
            </button>
          ) : showShellDenied ? (
            <button
              type="button"
              disabled
              className="inline-flex flex-1 cursor-not-allowed items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[11px] font-medium text-subtle"
              title="Exec requires Operator or Admin role"
              data-testid={`workload-card-shell-denied-${shortName}`}
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
