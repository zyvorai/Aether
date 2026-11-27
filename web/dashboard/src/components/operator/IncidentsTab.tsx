// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Activity, Download } from 'lucide-react';
import type { IncidentDetail, IncidentList, ObserveReport } from '../../types/operator';
import { apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { formatMinutes, formatSeconds, friendlyError } from '../../utils/operator';
import { useQueryParam } from '../../utils/urlState';
import Badge from '../Badge';
import EmptyState from '../EmptyState';
import { Button } from '../ui/Button';
import { ChangeList, downloadText, ErrorText, Loadable, Section, useApi } from './shared';

function status(i: { resolution: string | null }) {
  return i.resolution === null ? (
    <Badge text="open" variant="red" />
  ) : (
    <Badge text={i.resolution} variant={i.resolution === 'recovered' ? 'green' : 'muted'} />
  );
}

function Detail({ id, refreshKey, onChanged }: { id: string; refreshKey?: number; onChanged: () => void }) {
  const { data, error, loading, reload } = useApi<IncidentDetail>(`/incidents/${encodeURIComponent(id)}`, refreshKey);
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);

  async function send(path: string, body?: unknown): Promise<boolean> {
    setBusy(true);
    setFailed(null);
    const res = await apiPost(`/incidents/${encodeURIComponent(id)}/${path}`, body);
    setBusy(false);
    if (!res.success) {
      setFailed(friendlyError(res.error));
      return false;
    }
    void reload();
    onChanged();
    return true;
  }

  return (
    <Loadable loading={loading} error={error} hasData={data !== null} title="Incident unavailable" onRetry={() => void reload()}>
      {data ? (
        <div className="space-y-4" data-testid="incident-detail">
          <Section
            title={`Incident ${data.id}: ${data.workload}`}
            actions={
              <div className="flex flex-wrap gap-2">
                {data.acknowledged ? null : (
                  <Button size="sm" disabled={busy} onClick={() => void send('ack')}>
                    Acknowledge
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="secondary"
                  className="gap-1.5"
                  onClick={async () => {
                    const ok = await downloadText(`/incidents/${data.id}/report`, `incident-${data.id}.md`, 'text/markdown');
                    setFailed(ok ? null : 'The draft could not be downloaded.');
                  }}
                >
                  <Download className="h-4 w-4" /> Post-incident draft
                </Button>
              </div>
            }
          >
            <div className="flex flex-wrap items-center gap-2 text-sm text-muted">
              {status(data)}
              <span>started {formatTimestamp(data.opened_at)}</span>
              <span>· {formatSeconds(data.duration_secs)}</span>
              <span>· {data.failed_checks} failed checks</span>
              <span>· last state {data.last_state}</span>
              {data.last_message ? <span>({data.last_message})</span> : null}
              {data.restarts > 0 ? <span>· {data.restarts} restarts</span> : null}
            </div>
            {data.acknowledged ? (
              <p className="mt-1 text-sm text-muted">
                Acknowledged by {data.acknowledged.actor} at {formatTimestamp(data.acknowledged.at)}
              </p>
            ) : null}
            <ErrorText>{failed}</ErrorText>
          </Section>

          <Section title={`Changes in the ${formatSeconds(data.window_secs)} before it started`}>
            <p className="mb-2 text-sm text-muted">Correlation in time only; none of these is a confirmed cause.</p>
            {data.suspects.length === 0 ? (
              <p className="text-sm text-muted">No change was recorded in that window.</p>
            ) : (
              <ul className="space-y-2">
                {data.suspects.map((s, i) => (
                  <li key={`${s.change.at}-${i}`} className="text-sm">
                    <span className="font-medium text-foreground">{s.change.title}</span>{' '}
                    <span className="text-muted">
                      {formatSeconds(s.seconds_before)} before · {s.relation} {s.change.workload} · {s.change.detail}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Section>

          <Section title="Timeline">
            <ChangeList changes={data.timeline} empty="No recorded entries." />
          </Section>

          <Section title="Notes">
            <ul className="mb-3 space-y-1.5">
              {data.notes.map((n) => (
                <li key={n.at} className="text-sm text-muted">
                  <span className="font-mono text-xs">{formatTimestamp(n.at)}</span> {n.actor}:{' '}
                  <span className="text-foreground">{n.text}</span>
                </li>
              ))}
              {data.notes.length === 0 ? <li className="text-sm text-muted">No notes yet.</li> : null}
            </ul>
            <form
              className="flex flex-col gap-2 sm:flex-row"
              onSubmit={async (e) => {
                e.preventDefault();
                if (note.trim() && (await send('notes', { text: note }))) setNote('');
              }}
            >
              <input
                aria-label="New note"
                className="glass-input"
                maxLength={2000}
                placeholder="What did you find or do?"
                value={note}
                onChange={(e) => setNote(e.target.value)}
              />
              <Button type="submit" size="sm" disabled={busy || !note.trim()}>
                Add note
              </Button>
            </form>
          </Section>
        </div>
      ) : null}
    </Loadable>
  );
}

export default function IncidentsTab({ refreshKey }: { refreshKey?: number }) {
  const [selected, setSelected] = useQueryParam('incident');
  const { data, error, loading, reload } = useApi<IncidentList>('/incidents', refreshKey);
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<string | null>(null);

  async function checkNow() {
    setChecking(true);
    const res = await apiPost<ObserveReport>('/observe');
    setChecking(false);
    if (res.success && res.data) {
      const r = res.data;
      const errors = r.errors.length ? ` ${r.errors.length} could not be read: ${r.errors.join('; ')}` : '';
      setCheckResult(`Checked ${r.observed} workload(s): ${r.opened.length} opened, ${r.resolved.length} resolved.${errors}`);
      void reload();
    } else {
      setCheckResult(friendlyError(res.error));
    }
  }

  const checkButton = (
    <Button size="sm" variant="secondary" className="gap-1.5" disabled={checking} onClick={() => void checkNow()}>
      <Activity className="h-4 w-4" /> {checking ? 'Checking…' : 'Check now'}
    </Button>
  );

  return (
    <Loadable loading={loading} error={error} hasData={data !== null} title="Incidents unavailable" onRetry={() => void reload()}>
      {data ? (
        <div className="space-y-4">
          <Section title="Incidents" actions={checkButton}>
            <p className="mb-3 text-sm text-muted">
              Opened after {data.observation_threshold} consecutive failed health checks. Mean time to recovery:{' '}
              {formatMinutes(data.recovery.mean_minutes)}
              {data.recovery.recovered > 0 ? ` over ${data.recovery.recovered} recovered` : ''}.
            </p>
            {checkResult ? (
              <p role="status" className="mb-3 text-sm text-foreground">
                {checkResult}
              </p>
            ) : null}
            {data.incidents.length === 0 ? (
              <EmptyState icon={<Activity size={40} />} title="No incident recorded" description="Health checks run while aether serve is running." />
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full text-sm" data-testid="incident-table">
                  <thead className="text-left text-muted">
                    <tr>
                      {['ID', 'Workload', 'Status', 'Started', 'Duration', 'Acknowledged'].map((h) => (
                        <th key={h} className="px-2 py-2 text-xs font-semibold">
                          {h}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {data.incidents.map((i) => (
                      <tr key={i.id} className={`border-t border-border ${selected === String(i.id) ? 'bg-hover' : ''}`}>
                        <td className="px-2 py-2">
                          <button
                            type="button"
                            className="focus-ring text-primary hover:underline"
                            aria-label={`Open incident ${i.id}`}
                            onClick={() => setSelected(String(i.id))}
                          >
                            #{i.id}
                          </button>
                        </td>
                        <td className="px-2 py-2 font-mono text-xs">{i.workload}</td>
                        <td className="px-2 py-2">{status(i)}</td>
                        <td className="px-2 py-2 font-mono text-xs">{formatTimestamp(i.opened_at)}</td>
                        <td className="px-2 py-2">{formatSeconds(i.duration_secs)}</td>
                        <td className="px-2 py-2 text-muted">{i.acknowledged?.actor ?? '-'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </Section>
          {selected ? <Detail key={selected} id={selected} refreshKey={refreshKey} onChanged={() => void reload()} /> : null}
        </div>
      ) : null}
    </Loadable>
  );
}
