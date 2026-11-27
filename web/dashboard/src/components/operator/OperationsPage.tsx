// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useSearchParams } from 'react-router';
import { parseTab, type OperatorTab } from '../../utils/operator';
import { withAuroraPage } from '../layout/AuroraPage';
import SegmentedControl from '../ui/SegmentedControl';
import AttentionTab from './AttentionTab';
import ChangesTab from './ChangesTab';
import IncidentsTab from './IncidentsTab';
import ReportTab from './ReportTab';
import { useApi } from './shared';
import WorkloadTab from './WorkloadTab';

const TABS: { key: OperatorTab; label: string }[] = [
  { key: 'attention', label: 'Attention' },
  { key: 'changes', label: 'What changed' },
  { key: 'incidents', label: 'Incidents' },
  { key: 'workload', label: 'Workload' },
  { key: 'report', label: 'Report' },
];

function OperationsPage({ refreshKey }: { refreshKey?: number }) {
  const [params, setParams] = useSearchParams();
  const tab = parseTab(params.get('tab') ?? '');
  const list = useApi<{ name: string; source?: string }[]>('/workloads', refreshKey);
  // Only workloads Aether manages have revisions, holds and health history.
  const workloads = (list.data ?? [])
    .filter((w) => (w.source ?? 'aether') === 'aether')
    .map((w) => w.name)
    .sort();

  // Each tab owns its query parameters, so switching starts from a clean URL.
  function open(next: OperatorTab, extra: { workload?: string; incident?: string } = {}) {
    const query = new URLSearchParams({ tab: next });
    const workload = extra.workload ?? params.get('workload') ?? '';
    if (workload && next !== 'attention') query.set('workload', workload);
    if (extra.incident) query.set('incident', extra.incident);
    setParams(query);
  }

  return (
    <div className="space-y-5">
      <div className="max-w-full overflow-x-auto">
        <SegmentedControl value={tab} onChange={(next) => open(next)} items={TABS} />
      </div>
      {tab === 'attention' ? <AttentionTab refreshKey={refreshKey} onOpen={open} /> : null}
      {tab === 'changes' ? <ChangesTab refreshKey={refreshKey} workloads={workloads} /> : null}
      {tab === 'incidents' ? <IncidentsTab refreshKey={refreshKey} /> : null}
      {tab === 'workload' ? <WorkloadTab refreshKey={refreshKey} workloads={workloads} /> : null}
      {tab === 'report' ? <ReportTab refreshKey={refreshKey} /> : null}
    </div>
  );
}

export default withAuroraPage('operations', OperationsPage);
