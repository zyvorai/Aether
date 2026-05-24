import { useState, useEffect, useCallback } from 'react';
import { BellRing, Radio, Send, Plus, Trash2 } from 'lucide-react';
import { apiFetchSettled, apiPost, apiDelete } from '../../utils/api';
import PageToolbar from '../PageToolbar';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import type { AlertsStatus } from '../../types/api';

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

export default function AlertsPage() {
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

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<AlertsStatus>('/alerts/status');
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
  }, [testChannel]);

  useEffect(() => {
    void load();
  }, [load]);

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

  if (loading && !status && !loadFailed) {
    return <PageLoading rows={4} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Alerts unavailable" onRetry={() => void load()} />;
  }

  const channels = status?.channels ?? [];
  const rules = status?.rules ?? [];

  return (
    <div>
      <PageToolbar onRefresh={() => void load()} refreshing={loading} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="dash-card">
          <div className="flex items-center justify-between gap-3 mb-4">
            <div className="flex items-center gap-3">
              <Radio className="text-aether" size={20} />
              <h2 className="text-lg font-semibold text-slate-100">Notification channels</h2>
            </div>
            <button
              type="button"
              onClick={() => setShowAddChannel((v) => !v)}
              className="inline-flex items-center gap-1.5 rounded-lg border border-slate-700 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-800"
            >
              <Plus size={14} />
              Add webhook
            </button>
          </div>
          {showAddChannel && (
            <div className="mb-4 rounded-xl border border-slate-800 bg-slate-950/60 p-4 space-y-3">
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="Channel name"
                className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm"
              />
              <input
                value={newUrl}
                onChange={(e) => setNewUrl(e.target.value)}
                placeholder="Webhook URL"
                className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm"
              />
              <div className="flex flex-wrap gap-3">
                <select
                  value={newMethod}
                  onChange={(e) => setNewMethod(e.target.value)}
                  className="rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm"
                >
                  <option value="POST">POST</option>
                  <option value="PUT">PUT</option>
                </select>
                <select
                  value={newSeverity}
                  onChange={(e) => setNewSeverity(e.target.value)}
                  className="rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm"
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
                  className="px-4 py-2 bg-aether hover:bg-aether/90 disabled:opacity-50 rounded-lg text-sm font-medium text-white"
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
                  className="rounded-xl border border-slate-800 bg-slate-950/60 px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium text-slate-100">{ch.name}</span>
                    <div className="flex items-center gap-2">
                      <Badge text={ch.enabled ? 'enabled' : 'disabled'} variant={ch.enabled ? 'green' : 'muted'} />
                      <button
                        type="button"
                        onClick={() => void handleDeleteChannel(ch.name)}
                        disabled={channelDeleting === ch.name}
                        className="p-1.5 text-slate-500 hover:text-red-400 rounded"
                        title="Remove channel"
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </div>
                  <p className="text-sm text-slate-400 mt-1 capitalize">{ch.channel_type}</p>
                  <p className="text-xs text-slate-500 mt-1">
                    Min severity: {ch.min_severity}
                    {ch.categories.length > 0 ? ` · categories: ${ch.categories.join(', ')}` : ''}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="dash-card">
          <div className="flex items-center gap-3 mb-4">
            <BellRing className="text-amber-400" size={20} />
            <h2 className="text-lg font-semibold text-slate-100">Alert rules</h2>
          </div>
          {rules.length === 0 ? (
            <p className="text-sm text-slate-500">No alert rules configured.</p>
          ) : (
            <ul className="space-y-3 max-h-[28rem] overflow-auto">
              {rules.map((rule) => (
                <li
                  key={rule.name}
                  className="rounded-xl border border-slate-800 bg-slate-950/60 px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium text-slate-100">{rule.name}</span>
                    <Badge text={rule.enabled ? 'on' : 'off'} variant={rule.enabled ? 'green' : 'muted'} />
                  </div>
                  <p className="text-sm text-slate-400 mt-1">{rule.condition}</p>
                  <p className="text-xs text-slate-500 mt-1">
                    Severity {rule.severity} · cooldown {rule.cooldown_seconds}s
                    {rule.last_triggered ? ` · last: ${rule.last_triggered}` : ''}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <div className="dash-card mt-6">
        <div className="flex items-center gap-3 mb-4">
          <Send className="text-blue-400" size={20} />
          <h2 className="text-lg font-semibold text-slate-100">Test webhook</h2>
        </div>
        <p className="text-sm text-slate-500 mb-4">
          Emits a test event through the selected channel (same as <code className="text-slate-400">aether webhook test</code>).
        </p>
        <div className="flex flex-wrap gap-3 items-center">
          <select
            value={testChannel}
            onChange={(e) => setTestChannel(e.target.value)}
            className="rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 min-w-[12rem]"
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
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:opacity-50 rounded-xl text-sm font-medium text-white"
          >
            {testLoading ? 'Sending…' : 'Send test'}
          </button>
        </div>
      </div>
    </div>
  );
}
