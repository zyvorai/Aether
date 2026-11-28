// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { AlertTriangle, CheckCircle2, Info, OctagonAlert } from 'lucide-react';
import type { AttentionQueue } from '../../types/operator';
import { attentionTarget, levelTone, type OperatorTab } from '../../utils/operator';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import { Button } from '../ui/Button';
import { Loadable, Section, useApi } from './shared';

const LEVEL_MARK = {
  critical: { Icon: OctagonAlert, tone: 'pink' },
  warning: { Icon: AlertTriangle, tone: 'amber' },
  info: { Icon: Info, tone: 'sky' },
} as const;

export default function AttentionTab({
  refreshKey,
  onOpen,
}: {
  refreshKey?: number;
  onOpen: (tab: OperatorTab, params: { workload?: string; incident?: string }) => void;
}) {
  const { data, error, loading, reload } = useApi<AttentionQueue>('/attention', refreshKey);
  return (
    <Loadable loading={loading} error={error} hasData={data !== null} title="Attention queue unavailable" onRetry={() => void reload()}>
      {data && data.items.length === 0 ? (
        <EmptyState
          icon={<CheckCircle2 size={48} />}
          title="Nothing needs attention"
          description={`Checked ${data.workloads_checked} managed workload(s) against recorded data.`}
        />
      ) : data ? (
        <Section title={`${data.items.length} item(s) across ${data.workloads_checked} workload(s)`}>
          <ul className="divide-y divide-[var(--border)]" data-testid="attention-list">
            {data.items.map((item) => {
              const target = attentionTarget(item);
              const { Icon, tone } = LEVEL_MARK[item.level] ?? LEVEL_MARK.info;
              return (
                <li key={`${item.kind}-${item.workload}-${item.title}`} className="flex flex-col gap-2 py-4 sm:flex-row sm:items-start sm:gap-4">
                  <span className="level-mark" data-tone={tone} aria-hidden>
                    <Icon />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <Badge text={item.level} variant={levelTone(item.level)} />
                      <span className="font-medium text-foreground">{item.title}</span>
                      {item.workload ? <span className="font-mono text-xs text-muted">{item.workload}</span> : null}
                    </div>
                    <p className="mt-1 break-words text-sm text-muted">{item.detail}</p>
                    {item.next ? <code className="mt-1 block break-all text-xs text-subtle">{item.next}</code> : null}
                  </div>
                  {target ? (
                    <Button variant="secondary" size="sm" onClick={() => onOpen(target.tab, target)}>
                      Open
                    </Button>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </Section>
      ) : null}
    </Loadable>
  );
}
