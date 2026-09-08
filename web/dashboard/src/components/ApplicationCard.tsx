// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { Activity, FileText, Layers, RefreshCw, Rocket, Shield } from 'lucide-react';
import type { WorkloadResponse } from '../types/api';
import { applicationLabel, healthTone, workspaceLabel } from '../utils/k8sUx';
import Badge from './Badge';

interface ApplicationCardProps {
  app: WorkloadResponse;
  onOpen: () => void;
  onLogs: () => void;
  onScale?: () => void;
  onRestart?: () => void;
}

export default function ApplicationCard({ app, onOpen, onLogs, onScale, onRestart }: ApplicationCardProps) {
  const tone = healthTone(app.status);
  const border =
    tone === 'healthy'
      ? 'border-success/30 hover:border-success/50'
      : tone === 'critical'
        ? 'border-danger/40 hover:border-danger/60'
        : tone === 'stopped'
          ? 'glass-divider hover:border-primary/30'
          : 'border-warning/30 hover:border-warning/50';

  const statusVariant =
    tone === 'healthy' ? 'green' : tone === 'critical' ? 'red' : tone === 'stopped' ? 'muted' : 'yellow';

  return (
    <article
      className={`hub-link-card p-4 ${border}`}
      data-testid={`app-card-${applicationLabel(app)}`}
    >
      <button type="button" onClick={onOpen} className="w-full text-left">
        <div className="flex items-start justify-between gap-2">
          <div>
            <h3 className="text-base font-semibold text-foreground truncate">{applicationLabel(app)}</h3>
            <p className="text-xs text-subtle mt-0.5">
              {workspaceLabel(app.namespace)}
              {app.cluster ? ` · ${app.cluster}` : ''}
            </p>
          </div>
          <Badge text={app.status} variant={statusVariant} />
        </div>
        <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-muted">
          <div className="flex items-center gap-1.5">
            <Layers size={14} className="text-primary shrink-0" />
            <span className="truncate">{app.kind ?? 'Application'}</span>
          </div>
          <div className="flex items-center gap-1.5 truncate">
            <Activity size={14} className="shrink-0" />
            <span className="truncate">{app.image.split(':').pop() ?? app.image}</span>
          </div>
        </div>
      </button>
      <div className="mt-4 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onOpen}
          className="inline-flex items-center gap-1 rounded-lg bg-primary/15 px-2.5 py-1.5 text-xs font-medium text-primary hover:bg-primary/25"
        >
          <Rocket size={12} /> Open
        </button>
        <button
          type="button"
          onClick={onLogs}
          className="inline-flex items-center gap-1 quick-link-chip px-2.5 py-1.5 text-xs text-muted"
        >
          <FileText size={12} /> Logs
        </button>
        {onScale && (
          <button
            type="button"
            onClick={onScale}
            className="inline-flex items-center gap-1 quick-link-chip px-2.5 py-1.5 text-xs text-muted"
          >
            <Layers size={12} /> Scale
          </button>
        )}
        {onRestart && (
          <button
            type="button"
            onClick={onRestart}
            className="inline-flex items-center gap-1 quick-link-chip px-2.5 py-1.5 text-xs text-muted"
          >
            <RefreshCw size={12} /> Restart
          </button>
        )}
        {tone !== 'healthy' && (
          <span className="inline-flex items-center gap-1 text-xs text-warning ml-auto">
            <Shield size={12} /> Needs fix
          </span>
        )}
      </div>
    </article>
  );
}
