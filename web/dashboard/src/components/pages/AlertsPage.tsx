// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo } from 'react';
import { withAuroraPage } from '../layout/AuroraPage';
import { Link } from 'react-router';
import { BellRing, Radio, Send, Plus, Trash2, Webhook } from 'lucide-react';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { apiFetchSettled, apiPost, apiDelete } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import CardGrid from '../CardGrid';
import EntityCard from '../EntityCard';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { AlertsStatus } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function ruleMatchesWorkload(
  rule: { name: string; condition: string; message_template: string },
  workload: string,
): boolean {
  const needle = workload.toLowerCase();
  return (
    rule.name.toLowerCase().includes(needle) ||
    rule.condition.toLowerCase().includes(needle) ||
    rule.message_template.toLowerCase().includes(needle)
  );
}

function AlertsPage({ refreshKey }: { refreshKey?: number } = {}) {
  const [workloadQuery] = useQueryParam('workload', '');
  const [status, setStatus] = useState<AlertsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [testChannel, setTestChannel] = useState('');
  const [testLoading, setTestLoading] = useState(false);
  const [showAddChannel, setShowAddChannel] = useState(false);
  const [newName, setNewName] = useState('');
  const [newUrl, setNewUrl] = useState('');
  const [newMethod, setNewMethod] = useState('POST');
  const [newSeverity, setNewSeverity] = useState('warning');
  const [channelSaving, setChannelSaving] = useState(false);
  const [channelDeleting, setChannelDeleting] = useState<string | null>(null);
  const [queue, setQueue] = useState<Array<{ url: string; method: string; attempts: number; max_attempts: number; next_attempt_at: string }>>([]);
  const [queueLoading, setQueueLoading] = useState(false);
  const [flushLoading, setFlushLoading] = useState(false);

  const workloadFocus = workloadQuery.trim();

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const statusPath = workloadFocus
      ? `/alerts/status?workload=${encodeURIComponent(workloadFocus)}`
      : '/alerts/status';
    const result = await apiFetchSettled<AlertsStatus>(statusPath);
    if (!result.ok) {
      setLoadFailed(true);
      setStatus(null);
    } else {
      setStatus(result.data);
      if (result.data.channels?.length && !testChannel) {
        setTestChannel(result.data.channels[0].name);
      }
    }
    setLoading(false);
  }, [testChannel, workloadFocus]);

  useEffect(() => {
    void load();
  }, [load]);

  async function loadQueue() {
    setQueueLoading(true);
    const result = await apiFetchSettled<Array<{ url: string; method: string; attempts: number; max_attempts: number; next_attempt_at: string }>>('/webhooks/queue');
    setQueueLoading(false);
    if (result.ok) setQueue(result.data);
  }

  useEffect(() => {
    void loadQueue();
  }, [refreshKey]);

  async function handleFlushQueue() {
    setFlushLoading(true);
    const res = await apiPost<{ before?: number; after?: number; processed?: number }>('/webhooks/flush', {});
    setFlushLoading(false);
    if (res.success) {
      toast(`Processed queue (${res.data?.processed ?? 0} delivery attempt(s))`, 'success');
      void loadQueue();
    } else {
      toast(res.error ?? 'Flush failed', 'error');
    }
  }

  async function handleTestWebhook() {
    if (!testChannel) return;
    setTestLoading(true);
    const res = await apiPost('/webhooks/test', { channel: testChannel });
    setTestLoading(false);
    if (res.success) {
      toast(`Test event sent to channel "${testChannel}"`, 'success');
    } else {
      toast(res.error ?? 'Webhook test failed', 'error');
    }
  }

  async function handleCreateChannel() {
    if (!newName.trim() || !newUrl.trim()) {
      toast('Name and URL are required', 'error');
      return;
    }
    setChannelSaving(true);
    const res = await apiPost('/webhooks/channels', {
      name: newName.trim(),
      url: newUrl.trim(),
      method: newMethod,
      severity: newSeverity,
    });
    setChannelSaving(false);
    if (res.success) {
      toast(`Channel "${newName}" created`, 'success');
      setShowAddChannel(false);
      setNewName('');
      setNewUrl('');
      void load();
    } else {
      toast(res.error ?? 'Failed to create channel', 'error');
    }
  }

  async function handleDeleteChannel(name: string) {
    setChannelDeleting(name);
    const res = await apiDelete(`/webhooks/channels/${encodeURIComponent(name)}`, {
      label: `Remove notification channel "${name}"`,
    });
    setChannelDeleting(null);
    if (res.success) {
      toast(`Channel "${name}" removed`, 'success');
      if (testChannel === name) setTestChannel('');
      void load();
    } else {
      toast(res.error ?? 'Failed to remove channel', 'error');
    }
  }

  const channels = status?.channels ?? [];
  const allRules = status?.rules ?? [];
  const rules = useMemo(() => {
    if (!workloadFocus) return allRules;
    return allRules.filter((rule) => {
      const scoped = rule.workload?.trim();
      if (scoped) return scoped === workloadFocus;
      return ruleMatchesWorkload(rule, workloadFocus);
    });
  }, [allRules, workloadFocus]);

  if (loading && !status && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Alerts unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <WorkloadContextBanner
        testId="alerts-workload-context"
        workload={workloadFocus}
        description="Alert rules matching workload"
      >
        <WorkloadScopedCrossLinks
          workload={workloadFocus}
          prefix="alerts"
          showDrift
          showGitops
          showMetrics
        />
        {workloadFocus ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('policy'), { workload: workloadFocus })}
              className="text-brand hover:underline"
              data-testid="alerts-context-policy-link"
            >
              Policy →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('openapi'), { workload: workloadFocus })}
              className="text-brand hover:underline"
              data-testid="alerts-openapi-link"
            >
              OpenAPI →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('zyra'), { workload: workloadFocus, q: `Explain alerts for ${workloadFocus}` })}
              className="text-brand hover:underline"
              data-testid="alerts-context-copilot-link"
            >
              Copilot →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('secrets'), { workload: workloadFocus })}
              className="text-brand hover:underline"
              data-testid="alerts-context-secrets-link"
            >
              Secrets →
            </Link>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('rbac'), { workload: workloadFocus })}
              className="text-brand hover:underline"
              data-testid="alerts-context-rbac-link"
            >
              RBAC →
            </Link>
          </>
        ) : null}
      </WorkloadContextBanner>
      <PageToolbar
        onRefresh={() => {
          void load();
          void loadQueue();
        }}
        refreshing={loading || queueLoading}
      />

      <div className="mb-4">
        <Link
          to={
            workloadFocus
              ? pathWithQuery(viewToPath('events'), { workload: workloadFocus })
              : viewToPath('events')
          }
          className="text-xs text-brand hover:underline"
          data-testid="alerts-events-link"
        >
          View events feed →
        </Link>
        {' · '}
        <Link
          to={workloadFocus ? pathWithQuery(viewToPath('policy'), { workload: workloadFocus }) : viewToPath('policy')}
          className="text-xs text-brand hover:underline"
          data-testid="alerts-policy-link"
        >
          Policy check →
        </Link>
        {workloadFocus ? (
          <>
            {' · '}
            <Link
              to={pathWithQuery(viewToPath('workloads'), { workload: workloadFocus, tab: 'trust' })}
              className="text-xs text-brand hover:underline"
              data-testid="alerts-trust-link"
            >
              Trust & attestation →
            </Link>
          </>
        ) : null}
      </div>

      <section className="glass mb-6 p-6 sm:p-8">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="glass">
          <div className="flex items-center justify-between gap-3 mb-4">
            <div className="flex items-center gap-3">
              <Radio className="text-brand" size={20} />
              <h2 className="text-lg font-semibold text-ink">Notification channels</h2>
            </div>
            <button
              type="button"
              onClick={() => setShowAddChannel((v) => !v)}
              className="inline-flex items-center gap-1.5 btn-secondary px-3 py-1.5 text-xs font-medium"
            >
              <Plus size={14} />
              Add webhook
            </button>
          </div>
          {channels.length === 0 && !showAddChannel && (
            <p className="text-sm text-ink-3 mb-4">
              No channels configured.{' '}
              <Link to={viewToPath('platform')} className="text-brand hover:underline">
                Configure audit webhooks on Platform →
              </Link>
            </p>
          )}
          {showAddChannel && (
            <div data-testid="alerts-add-channel-form" className="glass-context-banner mb-4 p-4 space-y-3">
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="Channel name"
                className="glass-input"
              />
              <input
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                placeholder="Webhook URL"
                className="glass-input"
              />
              <div className="flex flex-wrap gap-3">
                <select
                  value={newMethod}
                  onChange={(e) => setNewMethod(e.target.value)}
                  className="glass-select"
                >
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                </select>
                <select
                  value={newSeverity}
                  onChange={(e) => setNewSeverity(e.target.value)}
                  className="glass-select"
                >
                  <option value="info">info</option>
                  <option value="warning">warning</option>
                  <option value="error">error</option>
                  <option value="critical">critical</option>
                </select>
                <button
                  type="button"
                  onClick={() => void handleCreateChannel()}
                  disabled={channelSaving}
                  className="btn-primary disabled:opacity-50"
                >
                  {channelSaving ? 'Saving…' : 'Create'}
                </button>
              </div>
            </div>
          )}
          {channels.length === 0 ? (
            <EmptyState
              icon={<BellRing size={40} />}
              title="No channels"
              description="Add a webhook channel above or use: aether webhook add &lt;name&gt;"
            />
          ) : (
            <ul className="space-y-3">
              {channels.map((ch) => (
                <li
                  key={ch.name}
                  className="glass px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium text-ink">{ch.name}</span>
                    <div className="flex items-center gap-2">
                      <Badge text={ch.enabled ? 'enabled' : 'disabled'} variant={ch.enabled ? 'green' : 'muted'} />
                      <button
                        type="button"
                        onClick={() => void handleDeleteChannel(ch.name)}
                        disabled={channelDeleting === ch.name}
                        className="p-1.5 text-ink-3 hover:text-red-400 rounded"
                        title="Remove channel"
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                  <p className="text-sm text-ink-2 mt-1 capitalize">{ch.channel_type}</p>
                  <p className="text-xs text-ink-3 mt-1">
                    Min severity: {ch.min_severity}
                    {ch.categories.length > 0 ? ` · categories: ${ch.categories.join(', ')}` : ''}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="glass" data-testid="alerts-rules-panel">
          <div className="flex items-center justify-between gap-3 mb-4">
            <div className="flex items-center gap-3">
              <BellRing className="text-amber-400" size={20} />
              <h2 className="text-lg font-semibold text-ink">Alert rules</h2>
            </div>
            <Link to={viewToPath('events')} className="text-xs text-brand hover:underline" data-testid="alerts-rules-events-link">
              Events feed →
            </Link>
          </div>
          {rules.length === 0 ? (
            <p className="text-sm text-ink-3">
              {workloadFocus
                ? `No alert rules mention "${workloadFocus}". Check the events feed for delivery history.`
                : 'No alert rules configured.'}
            </p>
          ) : (
            <ul className="space-y-3 max-h-[28rem] overflow-auto">
              {rules.map((rule) => (
                <li
                  key={rule.name}
                  className="glass px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-2 flex-wrap">
                    <span className="font-medium text-ink">{rule.name}</span>
                    <div className="flex items-center gap-2 flex-wrap">
                      {rule.workload ? (
                        <Link
                          to={pathWithQuery(viewToPath('workloads'), { workload: rule.workload, tab: 'trust' })}
                          className="text-xs font-mono text-brand hover:underline"
                          data-testid={`alerts-rule-workload-${rule.workload}`}
                        >
                          {rule.workload}
                        </Link>
                      ) : null}
                      <Badge text={rule.enabled ? 'on' : 'off'} variant={rule.enabled ? 'green' : 'muted'} />
                    </div>
                  </div>
                  <p className="text-sm text-ink-2 mt-1">{rule.condition}</p>
                  <p className="text-xs text-ink-3 mt-1">
                    Severity {rule.severity} · cooldown {rule.cooldown_seconds}s
                    {rule.last_triggered ? ` · last: ${rule.last_triggered}` : ''}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <div className="glass mt-6">
        <div className="flex items-center gap-3 mb-4">
          <Send className="text-blue-400" size={20} />
          <h2 className="text-lg font-semibold text-ink">Test webhook</h2>
        </div>
        <p className="text-sm text-ink-3 mb-4">
          Emits a test event through the selected channel (same as <code className="text-ink-2">aether webhook test</code>).
        </p>
        <div className="flex flex-wrap gap-3 items-center">
          <select
            value={testChannel}
            onChange={(e) => setTestChannel(e.target.value)}
            className="glass-select min-w-[12rem]"
          >
            {channels.map((ch) => (
              <option key={ch.name} value={ch.name}>
                {ch.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => void handleTestWebhook()}
            disabled={testLoading || !testChannel}
            data-testid="alerts-test-webhook"
            className="btn-primary disabled:opacity-50"
          >
            {testLoading ? 'Sending…' : 'Send test'}
          </button>
        </div>
      </div>

      <div className="glass mt-6" data-testid="alerts-webhook-queue">
        <div className="flex items-center justify-between gap-3 mb-4">
          <h2 className="text-lg font-semibold text-ink">Webhook retry queue</h2>
          <button
            type="button"
            data-testid="alerts-flush-queue"
            onClick={() => void handleFlushQueue()}
            disabled={flushLoading || queue.length === 0}
            className="btn-secondary disabled:opacity-50 text-xs"
          >
            {flushLoading ? 'Flushing…' : 'Flush queue'}
          </button>
        </div>
        {queue.length === 0 ? (
          <p className="text-sm text-ink-3">No pending webhook deliveries.</p>
        ) : (
          <CardGrid columns="compact">
            {queue.map((item, i) => (
              <EntityCard
                key={`${item.url}-${i}`}
                index={i}
                icon={<Webhook size={18} />}
                statusTone="amber"
                title={item.url}
                titleTooltip={item.url}
                subtitle={`${item.method} · next ${item.next_attempt_at.slice(0, 19)}`}
                body={
                  <span className="rounded-md glass-inset-surface border glass-divider px-2 py-0.5 text-[11px] text-ink-2">
                    Attempts {item.attempts}/{item.max_attempts}
                  </span>
                }
              />
            ))}
          </CardGrid>
        )}
      </div>
      </section>
    </div>
  );
}

export default withAuroraPage('alerts', AlertsPage);
