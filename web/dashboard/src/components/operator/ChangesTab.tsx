// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { ChangeFeed } from '../../types/operator';
import { queryString } from '../../utils/operator';
import { useQueryParam } from '../../utils/urlState';
import { ChangeList, Loadable, Section, useApi } from './shared';

const WINDOWS = ['1h', '24h', '7d', '30d'];

export default function ChangesTab({ refreshKey, workloads }: { refreshKey?: number; workloads: string[] }) {
  const [since, setSince] = useQueryParam('since', '24h');
  const [workload, setWorkload] = useQueryParam('workload');
  const { data, error, loading, reload } = useApi<ChangeFeed>(
    `/changes${queryString({ since, workload })}`,
    refreshKey,
  );
  const names = workload && !workloads.includes(workload) ? [workload, ...workloads] : workloads;

  return (
    <Section
      title="What changed"
      actions={
        <div className="flex flex-wrap gap-2">
          <select aria-label="Time window" className="glass-select !w-auto" value={since} onChange={(e) => setSince(e.target.value)}>
            {(WINDOWS.includes(since) ? WINDOWS : [since, ...WINDOWS]).map((w) => (
              <option key={w} value={w}>
                Last {w}
              </option>
            ))}
          </select>
          <select aria-label="Workload" className="glass-select !w-auto" value={workload} onChange={(e) => setWorkload(e.target.value)}>
            <option value="">All workloads</option>
            {names.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </div>
      }
    >
      <Loadable loading={loading} error={error} hasData={data !== null} title="Changes unavailable" onRetry={() => void reload()}>
        {data ? (
          <>
            {data.unavailable.map((u) => (
              <p key={u} role="alert" className="mb-2 text-sm text-warning">
                Incomplete: could not read {u}
              </p>
            ))}
            <ChangeList changes={data.changes} empty="Nothing was recorded in this window. Only changes made through Aether are recorded." />
            {data.truncated ? (
              <p className="mt-3 text-sm text-muted">
                Showing the newest {data.changes.length} of {data.total}.
              </p>
            ) : null}
          </>
        ) : null}
      </Loadable>
    </Section>
  );
}
