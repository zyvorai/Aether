// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Layers } from 'lucide-react';
import type { Explanation, Hold, PlanReport, Preview, RevisionSummary } from '../../types/operator';
import { apiPost } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import {
  buildPreviewAction,
  formatMinutes,
  formatReadyPct,
  friendlyError,
  levelTone,
  queryString,
  verdictTone,
  workloadPath,
  type PreviewKind,
} from '../../utils/operator';
import { useQueryParam } from '../../utils/urlState';
import Badge, { RuntimeBadge } from '../Badge';
import DestructiveConfirmDialog from '../DestructiveConfirmDialog';
import EmptyState from '../EmptyState';
import { Button } from '../ui/Button';
import PlanTable from './PlanTable';
import { ChangeList, ErrorText, Loadable, Section, useApi } from './shared';

function ExplainSection({ e }: { e: Explanation }) {
  return (
    <Section title="Explain" actions={<RuntimeBadge runtime={e.runtime} />}>
      <p className="text-sm text-foreground" data-testid="explain-summary">
        {e.summary}
      </p>
      <dl className="mt-3 grid gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
        {[
          ['Image', e.image],
          ['Spec in effect', e.spec_source ?? 'unavailable'],
          ['Last check', e.health.observed_at ? `${formatTimestamp(e.health.observed_at)} · ${e.health.state}` : 'none recorded'],
          ['Ready, last 24h', `${formatReadyPct(e.health.ready_pct_24h)}${e.health.checks_24h ? ` over ${e.health.checks_24h} checks` : ''}`],
          [
            'Incidents',
            e.recovery.mean_minutes === null
              ? `${e.recovery.incidents}`
              : `${e.recovery.incidents} · mean recovery ${formatMinutes(e.recovery.mean_minutes)}`,
          ],
          ['Depends on', e.dependencies.join(', ') || 'nothing declared'],
          ['Needed by', e.dependents.join(', ') || 'nothing declared'],
        ].map(([term, value]) => (
          <div key={term} className="min-w-0">
            <dt className="text-xs text-muted">{term}</dt>
            <dd className="break-words text-foreground">{value}</dd>
          </div>
        ))}
      </dl>
      {e.health.error ? <ErrorText>Status could not be read: {e.health.error}</ErrorText> : null}
      {e.attention.length > 0 ? (
        <ul className="mt-3 space-y-1.5">
          {e.attention.map((a) => (
            <li key={a.kind + a.title} className="flex flex-wrap items-center gap-2 text-sm">
              <Badge text={a.level} variant={levelTone(a.level)} />
              <span className="text-foreground">{a.title}</span>
              <span className="text-muted">{a.detail}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {e.decision ? (
        <details className="mt-3">
          <summary className="focus-ring cursor-pointer text-sm text-primary">Decision rules</summary>
          <ul className="mt-2 space-y-1 text-sm">
            {e.decision.rules.map((r) => (
              <li key={r.rule} className={r.applied ? 'text-foreground' : 'text-muted'}>
                {r.applied ? '→ ' : ''}
                <span className="font-medium">{r.rule}</span>: {r.detail}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
      {e.ignored_fields.length > 0 ? (
        <details className="mt-2">
          <summary className="focus-ring cursor-pointer text-sm text-primary">
            {e.ignored_fields.length} spec field(s) {e.runtime} does not fully honour
          </summary>
          <ul className="mt-2 space-y-1 text-sm text-muted">
            {e.ignored_fields.map((f) => (
              <li key={f.path}>
                <span className="font-mono text-xs text-foreground">{f.path}</span> {f.support}: {f.note}
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </Section>
  );
}

function HoldSection({ name, hold, onChanged }: { name: string; hold: Hold | null; onChanged: () => void }) {
  const [reason, setReason] = useState('');
  const [duration, setDuration] = useState('1h');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function send(path: string, body: unknown) {
    setBusy(true);
    setError(null);
    const res = await apiPost(path, body);
    setBusy(false);
    if (res.success) {
      setReason('');
      onChanged();
    } else {
      setError(friendlyError(res.error));
    }
  }

  return (
    <Section title="Change hold">
      {hold ? (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="text-sm text-foreground">
            On hold: {hold.reason} <span className="text-muted">(by {hold.actor}, until {hold.expires_at ? formatTimestamp(hold.expires_at) : 'released'})</span>
          </p>
          <Button size="sm" variant="secondary" disabled={busy} onClick={() => void send('/holds/release', { workload: name })}>
            Release
          </Button>
        </div>
      ) : (
        <form
          className="flex flex-col gap-2 sm:flex-row"
          onSubmit={(e) => {
            e.preventDefault();
            void send('/holds', { workload: name, reason, duration });
          }}
        >
          <input aria-label="Reason for the hold" className="glass-input" maxLength={500} placeholder="Why should nothing change?" value={reason} onChange={(e) => setReason(e.target.value)} />
          <select aria-label="Hold duration" className="glass-select sm:!w-44" value={duration} onChange={(e) => setDuration(e.target.value)}>
            <option value="1h">For 1 hour</option>
            <option value="8h">For 8 hours</option>
            <option value="24h">For 24 hours</option>
            <option value="7d">For 7 days</option>
            <option value="">Until released</option>
          </select>
          <Button type="submit" size="sm" disabled={busy || !reason.trim()}>
            Place hold
          </Button>
        </form>
      )}
      <p className="mt-2 text-xs text-muted">
        Aether refuses deploy, update, stop, restart, scale, migrate, rollback and delete while held. It does not stop changes made directly with kubectl or podman.
      </p>
      <ErrorText>{error}</ErrorText>
    </Section>
  );
}

function PreviewResult({ p }: { p: Preview }) {
  const list = (title: string, lines: string[], className: string) =>
    lines.length > 0 ? (
      <div>
        <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        <ul className={`list-disc space-y-0.5 pl-5 text-sm ${className}`}>
          {lines.map((l) => (
            <li key={l} className="break-words">
              {l}
            </li>
          ))}
        </ul>
      </div>
    ) : null;
  return (
    <div className="mt-4 space-y-3" data-testid="preview-result" role="status">
      <div className="flex items-center gap-2">
        <Badge text={p.verdict} variant={verdictTone(p.verdict)} />
        <span className="text-sm text-muted">
          {p.action} on {p.runtime}. Nothing was changed.
        </span>
      </div>
      {list('Blockers', p.blockers, 'text-danger')}
      {list('Warnings', p.warnings, 'text-warning')}
      {list('What it does', p.effects, 'text-muted')}
      {p.plan ? <PlanTable plan={p.plan} /> : null}
    </div>
  );
}

const KINDS: { value: PreviewKind; label: string }[] = [
  { value: 'update', label: 'Update the spec' },
  { value: 'restart', label: 'Restart' },
  { value: 'stop', label: 'Stop' },
  { value: 'migrate', label: 'Migrate' },
  { value: 'rollback', label: 'Roll back' },
  { value: 'delete', label: 'Delete' },
];

function PreviewSection({ name }: { name: string }) {
  const [kind, setKind] = useState<PreviewKind>('update');
  const [yaml, setYaml] = useState('');
  const [target, setTarget] = useState('');
  const [revision, setRevision] = useState('');
  const [result, setResult] = useState<Preview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run() {
    const built = buildPreviewAction({ kind, yaml, target, revision });
    setResult(null);
    if ('error' in built) {
      setError(built.error);
      return;
    }
    setError(null);
    setBusy(true);
    const res = await apiPost<Preview>(workloadPath(name, 'preview'), built.action);
    setBusy(false);
    if (res.success && res.data) setResult(res.data);
    else setError(friendlyError(res.error));
  }

  return (
    <Section title="Preview before you act">
      <div className="flex flex-col gap-2 sm:flex-row">
        <select
          aria-label="Action to preview"
          className="glass-select sm:!w-56"
          value={kind}
          onChange={(e) => {
            setKind(e.target.value as PreviewKind);
            setResult(null);
            setError(null);
          }}
        >
          {KINDS.map((k) => (
            <option key={k.value} value={k.value}>
              {k.label}
            </option>
          ))}
        </select>
        {kind === 'migrate' ? (
          <select aria-label="Target runtime" className="glass-select sm:!w-48" value={target} onChange={(e) => setTarget(e.target.value)}>
            <option value="">Target runtime…</option>
            <option value="podman">Podman</option>
            <option value="kubernetes">Kubernetes</option>
            <option value="kubevirt">KubeVirt</option>
          </select>
        ) : null}
        {kind === 'rollback' ? (
          <input aria-label="Revision" className="glass-input sm:!w-56" inputMode="numeric" placeholder="Revision (blank = default)" value={revision} onChange={(e) => setRevision(e.target.value)} />
        ) : null}
        <Button size="sm" disabled={busy} onClick={() => void run()}>
          {busy ? 'Previewing…' : 'Preview'}
        </Button>
      </div>
      {kind === 'update' ? (
        <textarea
          aria-label="Edited spec (YAML)"
          className="glass-input mt-2 min-h-48 w-full font-mono text-xs"
          placeholder="Paste the edited workload YAML"
          spellCheck={false}
          value={yaml}
          onChange={(e) => setYaml(e.target.value)}
        />
      ) : null}
      <ErrorText>{error}</ErrorText>
      {result ? <PreviewResult p={result} /> : null}
    </Section>
  );
}

function HistorySection({ name, refreshKey, held, onChanged }: { name: string; refreshKey?: number; held: boolean; onChanged: () => void }) {
  const { data, error, loading, reload } = useApi<RevisionSummary[]>(workloadPath(name, 'revisions'), refreshKey);
  const latest = data?.length ? data[data.length - 1].id : null;
  const [compare, setCompare] = useState<number | null>(null);
  const diff = useApi<PlanReport>(
    compare !== null && latest !== null
      ? workloadPath(name, `revision-diff${queryString({ from: latest, to: compare })}`)
      : null,
  );
  const [confirm, setConfirm] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);

  async function rollBack(revision: number) {
    setConfirm(null);
    setBusy(true);
    setMessage(null);
    setFailed(null);
    const res = await apiPost(workloadPath(name, 'rollback'), { revision });
    setBusy(false);
    if (res.success) {
      setMessage(`Rolled back to revision ${revision}.`);
      setCompare(null);
    } else {
      setFailed(friendlyError(res.error));
    }
    void reload();
    onChanged();
  }

  return (
    <Section title="Applied revisions">
      <Loadable loading={loading} error={error} hasData={data !== null} title="History unavailable" onRetry={() => void reload()}>
        {data && data.length === 0 ? (
          <p className="text-sm text-muted">No applied revision is recorded yet. Revisions are recorded from the next deploy or update.</p>
        ) : (
          <ul className="divide-y divide-[var(--border)]" data-testid="revision-list">
            {[...(data ?? [])].reverse().map((r) => (
              <li key={r.id} className="flex flex-col gap-2 py-2.5 sm:flex-row sm:items-center sm:justify-between">
                <div className="min-w-0 text-sm">
                  <span className="font-medium text-foreground">
                    #{r.id} {r.op} on {r.runtime}
                  </span>{' '}
                  <span className="text-muted">
                    {formatTimestamp(r.at)} by {r.actor}
                    {r.changed_fields !== null ? ` · ${r.changed_fields} field(s) changed` : ''}
                    {r.reason ? ` · ${r.reason}` : ''}
                  </span>
                </div>
                {r.id !== latest ? (
                  <div className="flex gap-2">
                    <Button size="sm" variant="ghost" aria-expanded={compare === r.id} onClick={() => setCompare(compare === r.id ? null : r.id)}>
                      Compare
                    </Button>
                    <Button size="sm" variant="secondary" disabled={busy || held} title={held ? 'Release the hold first' : undefined} onClick={() => setConfirm(r.id)}>
                      Roll back
                    </Button>
                  </div>
                ) : (
                  <Badge text="current" variant="green" />
                )}
              </li>
            ))}
          </ul>
        )}
      </Loadable>
      {compare !== null ? (
        <div className="mt-3">
          <p className="mb-2 text-sm text-muted">
            What rolling back from #{latest} to #{compare} would change:
          </p>
          <ErrorText>{diff.error}</ErrorText>
          {diff.data ? <PlanTable plan={diff.data} /> : null}
        </div>
      ) : null}
      {message ? (
        <p role="status" className="mt-2 text-sm text-success">
          {message}
        </p>
      ) : null}
      <ErrorText>{failed}</ErrorText>
      <DestructiveConfirmDialog
        isOpen={confirm !== null}
        onClose={() => setConfirm(null)}
        onConfirm={() => confirm !== null && void rollBack(confirm)}
        title={`Roll back to revision ${confirm}`}
        resourceName={name}
        confirmLabel="Roll back"
        description="The recorded spec is applied again. The workload may be replaced, and volume data is not restored."
      />
    </Section>
  );
}

export default function WorkloadTab({ refreshKey, workloads }: { refreshKey?: number; workloads: string[] }) {
  const [selected, setSelected] = useQueryParam('workload');
  const { data, error, loading, reload } = useApi<Explanation>(selected ? workloadPath(selected, 'explain') : null, refreshKey);
  const [version, setVersion] = useState(0);
  const names = selected && !workloads.includes(selected) ? [selected, ...workloads] : workloads;
  const changed = () => {
    setVersion((v) => v + 1);
    void reload();
  };

  return (
    <div className="space-y-4">
      <label className="block max-w-md text-sm text-muted">
        Managed workload
        <select className="glass-select mt-1" value={selected} onChange={(e) => setSelected(e.target.value)}>
          <option value="">Choose a workload…</option>
          {names.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
      </label>
      {!selected ? (
        <EmptyState icon={<Layers size={40} />} title="Choose a workload" description="See why it runs where it does, what changed, and preview your next action." />
      ) : (
        <Loadable loading={loading} error={error} hasData={data !== null} title="Workload unavailable" onRetry={() => void reload()}>
          {data && data.workload === selected ? (
            <div className="space-y-4" key={selected}>
              <ExplainSection e={data} />
              <HoldSection name={selected} hold={data.hold} onChanged={changed} />
              <PreviewSection name={selected} />
              <HistorySection name={selected} refreshKey={(refreshKey ?? 0) + version} held={data.hold !== null} onChanged={changed} />
              <Section title="Recent changes">
                <ChangeList changes={data.recent_changes} empty="Nothing recorded in the last 30 days." />
              </Section>
            </div>
          ) : null}
        </Loadable>
      )}
    </div>
  );
}
