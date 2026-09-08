// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import AuroraPage from '../layout/AuroraPage';
import { useState, useEffect, useRef, type MouseEvent } from 'react';
import { useNavigate, useSearchParams } from 'react-router';
import { Play, Square, Trash2, FileText, ClipboardCheck, Inbox, Plus, Rocket, FileCode2, Layers, Terminal, Info, Star, Download, LayoutGrid, List } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost, apiDelete, apiPut } from '../../utils/api';
import { useQueryParam } from '../../utils/urlState';
import { viewToPath } from '../../utils/dashboardRoutes';
import { markSpecValidated, markFirstDeploy, syncDeployFromWorkloads } from '../../utils/onboardingState';
import { useAuth } from '../../contexts/AuthContext';
import { DEFAULT_DEPLOY_WORKLOAD_YAML, freshDeployWorkloadYaml, workloadJsonToYaml, workloadNameFromYaml } from '../../utils/workloadYaml';
import {
  countAetherManaged,
  isAetherManaged,
  parseCreatedWorkloadName,
  sortWorkloadsForDisplay,
} from '../../utils/workloadFilters';
import { clusterResourceName, hasClusterLogs, isShellableClusterKind } from '../../utils/clusterExec';
import { getPinnedWorkloads, togglePinnedWorkload } from '../../utils/pinnedWorkloads';
import { downloadTextFile, workloadsToCsv } from '../../utils/workloadCsv';
import Badge, { RuntimeBadge } from '../Badge';
import StatRibbon from '../StatRibbon';
import SegmentedControl from '../ui/SegmentedControl';
import { categoricalDotClass } from '../../utils/categoricalColor';
import DataTable, { type DataTableColumn } from '../ui/DataTable';
import Modal from '../Modal';
import YamlInput from '../YamlInput';
import ValidateResultPanel from '../ValidateResultPanel';
import DeploySuccessPanel from '../DeploySuccessPanel';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, ValidateResponse, MigrationAdvice, PolicyResult } from '../../types/api';
import PageToolbar from '../PageToolbar';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import WorkloadDetail, { type DetailTab } from '../WorkloadDetail';
import WorkloadCard from '../WorkloadCard';

interface WorkloadsPageProps {
  initialSelectedName?: string | null;
  onClearInitialSelection?: () => void;
  /** Bumped by App.tsx on workloadChanged/healthUpdate SSE events — triggers a
   * data refresh. Deliberately NOT used as this page's React `key`: doing so
   * would fully remount the page (and any open WorkloadDetail) on every such
   * event, silently dropping in-progress Exec/port-forward sessions, open
   * modals, and filters. */
  refreshKey?: number;
}

function getStatusVariant(status: string): 'green' | 'red' | 'yellow' | 'muted' {
  const s = status.toLowerCase();
  if (s === 'running' || s === 'healthy' || s.includes('deployed')) return 'green';
  if (s === 'error' || s === 'failed') return 'red';
  if (s === 'stopped' || s === 'exited') return 'muted';
  return 'yellow';
}

function toast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

function openDeployModal(
  setDeployYaml: (yaml: string) => void,
  setDeployModal: (open: boolean) => void,
) {
  setDeployYaml(freshDeployWorkloadYaml());
  setDeployModal(true);
}

function WorkloadsPage({ initialSelectedName, onClearInitialSelection, refreshKey }: WorkloadsPageProps) {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const [workloads, setWorkloads] = useState<WorkloadResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useQueryParam('q');
  const [sourceFilter, setSourceFilter] = useQueryParam('source', 'all');
  const [kindFilter, setKindFilter] = useQueryParam('kind', 'all');
  const [clusterFilter, setClusterFilter] = useQueryParam('cluster', 'all');
  const [namespaceFilter, setNamespaceFilter] = useQueryParam('namespace', 'all');
  const [statusFilter, setStatusFilter] = useQueryParam('status', 'all');
  const [workloadParam, setWorkloadParam] = useQueryParam('workload');
  const [tabParam, setTabParam] = useQueryParam('tab');
  const [, setSearchParams] = useSearchParams();
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());
  const [updateModal, setUpdateModal] = useState<string | null>(null);
  const [updateLoading, setUpdateLoading] = useState(false);
  const [migrateModal, setMigrateModal] = useState<string | null>(null);
  const [migrateAdvice, setMigrateAdvice] = useState<MigrationAdvice | null>(null);
  const [migrateTarget, setMigrateTarget] = useState<string | null>(null);
  const [migrateStrategy, setMigrateStrategy] = useState('blue-green');
  const [migrateResult, setMigrateResult] = useState<string | null>(null);
  const [adviceLoading, setAdviceLoading] = useState(false);
  const [validateModal, setValidateModal] = useState(false);
  const [validateResult, setValidateResult] = useState<ValidateResponse | null>(null);
  const [validateLoading, setValidateLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [confirmAction, setConfirmAction] = useState<{ type: 'delete' | 'stop'; name: string } | null>(null);
  const [bulkConfirmDelete, setBulkConfirmDelete] = useState(false);
  const [deployModal, setDeployModal] = useState(false);
  const [deployYaml, setDeployYaml] = useState(DEFAULT_DEPLOY_WORKLOAD_YAML);
  const [deployLoading, setDeployLoading] = useState(false);
  const [deployTemplateLoading, setDeployTemplateLoading] = useState(false);
  const [deployValidateResult, setDeployValidateResult] = useState<ValidateResponse | null>(null);
  const [deployPolicyResult, setDeployPolicyResult] = useState<PolicyResult | null>(null);
  const [deployInlineValidateLoading, setDeployInlineValidateLoading] = useState(false);
  const [deploySuccess, setDeploySuccess] = useState<{ name: string; status: string } | null>(null);
  const [pendingSelect, setPendingSelect] = useState<{ name: string; tab: DetailTab } | null>(null);
  const [selectedWorkload, setSelectedWorkload] = useState<WorkloadResponse | null>(null);
  const [detailInitialTab, setDetailInitialTab] = useState<DetailTab>('overview');
  const [detailInitialShell, setDetailInitialShell] = useState(false);
  const [pinnedNames, setPinnedNames] = useState<string[]>(() => getPinnedWorkloads());
  const [pinnedOnly, setPinnedOnly] = useState(false);
  const [viewMode, setViewMode] = useState<'cards' | 'table'>(() => {
    try {
      return localStorage.getItem('aether_workloads_view') === 'cards' ? 'cards' : 'table';
    } catch {
      return 'table';
    }
  });
  const detailRef = useRef<HTMLDivElement | null>(null);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt'];

  function setViewModePersist(mode: 'cards' | 'table') {
    setViewMode(mode);
    try {
      localStorage.setItem('aether_workloads_view', mode);
    } catch { /* ignore */ }
  }

  function togglePin(name: string) {
    setPinnedNames(togglePinnedWorkload(name));
  }

  async function load() {
    setLoading(true);
    setLoadFailed(false);
    const result = await apiFetchSettled<WorkloadResponse[]>('/workloads');
    if (!result.ok) {
      setLoadFailed(true);
      setWorkloads([]);
    } else {
      setWorkloads(result.data);
      syncDeployFromWorkloads(result.data);
    }
    setLoading(false);
  }

  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional: refetch in place on the SSE signal, without remounting the page (see refreshKey prop doc).
  useEffect(() => { load(); }, [refreshKey]);

  // K8s-only clusters: avoid hiding discovered workloads when URL still has ?source=aether
  useEffect(() => {
    if (loading || loadFailed || workloads.length === 0) return;
    const aether = countAetherManaged(workloads);
    if (sourceFilter === 'aether' && aether === 0 && workloads.length > 0) {
      setSourceFilter('all');
    }
  }, [loading, loadFailed, workloads, sourceFilter, setSourceFilter]);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    if (params.get('deploy') === '1') {
      setDeployYaml(freshDeployWorkloadYaml());
      setDeployModal(true);
      params.delete('deploy');
      const qs = params.toString();
      window.history.replaceState(null, '', qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
    }
    if (params.get('validate') === '1') {
      setValidateModal(true);
      params.delete('validate');
      const qs = params.toString();
      window.history.replaceState(null, '', qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
    }
    const template = params.get('template');
    if (template) {
      setDeployModal(true);
      setDeployTemplateLoading(true);
      setDeployYaml(DEFAULT_DEPLOY_WORKLOAD_YAML);
      params.delete('template');
      const qs = params.toString();
      window.history.replaceState(null, '', qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
      void apiPost<Record<string, unknown>>(`/templates/${encodeURIComponent(template)}`, {})
        .then((res) => {
          if (res.success && res.data) {
            setDeployYaml(workloadJsonToYaml(res.data));
          }
        })
        .finally(() => setDeployTemplateLoading(false));
    }
  }, []);

  useEffect(() => {
    if (!pendingSelect || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === pendingSelect.name);
    if (!match) return;
    openWorkloadDetail(match, pendingSelect.tab);
    setPendingSelect(null);
  }, [pendingSelect, workloads]);

  useEffect(() => {
    if (!deploySuccess) return;
    const workloadName = deploySuccess.name;
    let cancelled = false;

    async function pollStatus() {
      const data = await apiFetch<WorkloadResponse[]>('/workloads');
      if (cancelled || !data) return;
      const match = data.find((w) => w.name === workloadName);
      if (match) {
        setDeploySuccess((prev) => (prev ? { ...prev, status: match.status } : null));
      }
    }

    void pollStatus();
    const interval = window.setInterval(pollStatus, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [deploySuccess?.name]);

  useEffect(() => {
    if (!initialSelectedName || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === initialSelectedName);
    if (match) {
      setDetailInitialTab('overview');
      setSelectedWorkload(match);
      setWorkloadParam(match.name);
    }
    onClearInitialSelection?.();
  }, [initialSelectedName, workloads, onClearInitialSelection, setWorkloadParam]);

  useEffect(() => {
    if (!workloadParam || workloads.length === 0) return;
    const match = workloads.find((w) => w.name === workloadParam);
    if (!match) return;
    const tab = (['overview', 'logs', 'topology', 'manifest', 'drift', 'scoring', 'events'] as const).includes(
      tabParam as DetailTab,
    )
      ? (tabParam as DetailTab)
      : 'overview';
    setDetailInitialTab(tab);
    setSelectedWorkload(match);
  }, [workloadParam, tabParam, workloads]);

  useEffect(() => {
    if (!selectedWorkload) return;
    // Bring the detail panel into view — previously it rendered under the full inventory table.
    window.requestAnimationFrame(() => {
      detailRef.current?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }, [selectedWorkload?.name, detailInitialTab]);

  async function handleAction(name: string, action: string) {
    setActionLoading(`${name}-${action}`);
    let res;
    if (action === 'start') {
      res = await apiPost(`/workloads/${name}/start`);
    } else if (action === 'stop') {
      res = await apiPost(`/workloads/${name}/stop`);
    } else if (action === 'delete') {
      res = await apiDelete(`/workloads/${name}`, { label: `Delete workload "${name}"` });
    }
    setActionLoading(null);
    if (res?.success) {
      toast(`${action.charAt(0).toUpperCase() + action.slice(1)} "${name}" succeeded`, 'success');
    } else {
      toast(`${action.charAt(0).toUpperCase() + action.slice(1)} "${name}" failed: ${res?.error ?? 'unknown error'}`, 'error');
    }
    load();
  }

  async function loadMigrationAdvice(name: string, target: string) {
    setAdviceLoading(true);
    setMigrateTarget(target);
    const data = await apiFetch<MigrationAdvice>(
      `/ai/migration-advice/${encodeURIComponent(name)}/${encodeURIComponent(target)}`,
    );
    setMigrateAdvice(data);
    if (data?.recommended_strategy) {
      const s = data.recommended_strategy.toLowerCase().replace(/_/g, '-');
      if (s.includes('immediate')) setMigrateStrategy('immediate');
      else if (s.includes('rolling')) setMigrateStrategy('rolling');
      else if (s.includes('canary')) setMigrateStrategy('canary');
      else setMigrateStrategy('blue-green');
    }
    setAdviceLoading(false);
  }

  async function handleMigrate(name: string, target: string, strategy: string) {
    setActionLoading(`${name}-migrate`);
    setMigrateResult(null);
    window.dispatchEvent(
      new CustomEvent('aether-live-activity', {
        detail: {
          workload: name,
          phase: 'running',
          percent: 10,
          eta_secs: 120,
          message: `Migrating ${name} → ${target}`,
        },
      }),
    );
    const res = await apiPost<string>(`/workloads/${name}/migrate`, { target_runtime: target, strategy });
    setActionLoading(null);
    if (res.success) {
      setMigrateResult(typeof res.data === 'string' ? res.data : 'Migration completed');
      toast(`Migration of "${name}" to ${target} completed`, 'success');
      void load();
    } else {
      setMigrateResult(res.error ?? 'Migration failed');
      toast(`Migration of "${name}" failed: ${res.error ?? 'unknown error'}`, 'error');
    }
  }

  async function handleValidate(yaml: string) {
    setValidateLoading(true);
    const result = await apiPost<ValidateResponse>('/validate', { yaml });
    setValidateResult(result.data ?? null);
    setValidateLoading(false);
    if (result.data?.valid) {
      markSpecValidated();
    }
  }

  function closeDeployModal() {
    setDeployModal(false);
    setDeployYaml(DEFAULT_DEPLOY_WORKLOAD_YAML);
    setDeployValidateResult(null);
    setDeployPolicyResult(null);
    setDeploySuccess(null);
    setDeployTemplateLoading(false);
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        copy.delete('deploy');
        return copy;
      },
      { replace: true },
    );
  }

  function finishDeploySuccess(tab: DetailTab) {
    if (!deploySuccess) return;
    setPendingSelect({ name: deploySuccess.name, tab });
    closeDeployModal();
  }

  function openWorkloadDetail(workload: WorkloadResponse, tab: DetailTab, openShell = false) {
    setDetailInitialShell(openShell);
    setDetailInitialTab(tab);
    setSelectedWorkload(workload);
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        copy.set('workload', workload.name);
        copy.delete('deploy');
        if (tab !== 'overview') {
          copy.set('tab', tab);
        } else {
          copy.delete('tab');
        }
        return copy;
      },
      { replace: true },
    );
  }

  function closeWorkloadDetail() {
    setSelectedWorkload(null);
    setDetailInitialShell(false);
    setSearchParams(
      (prev) => {
        const copy = new URLSearchParams(prev);
        copy.delete('workload');
        copy.delete('tab');
        return copy;
      },
      { replace: true },
    );
  }

  async function handleDeployValidate(yaml: string) {
    setDeployInlineValidateLoading(true);
    const [validateRes, policyRes] = await Promise.all([
      apiPost<ValidateResponse>('/validate', { yaml }),
      apiPost<PolicyResult>('/policy/check', { yaml }),
    ]);
    setDeployValidateResult(validateRes.data ?? null);
    setDeployPolicyResult(policyRes.data ?? null);
    setDeployInlineValidateLoading(false);
    if (validateRes.data?.valid) {
      markSpecValidated();
    }
  }

  async function handleDeploy(yaml: string) {
    if (!canMutate) {
      toast('Read-only session — deploy is disabled', 'error');
      return;
    }
    setDeployLoading(true);
    setDeployValidateResult(null);
    setDeployPolicyResult(null);

    const validateRes = await apiPost<ValidateResponse>('/validate', { yaml });
    setDeployValidateResult(validateRes.data ?? null);
    if (!validateRes.data?.valid) {
      setDeployLoading(false);
      toast('Fix validation errors before deploying', 'error');
      return;
    }

    const policyRes = await apiPost<PolicyResult>('/policy/check', { yaml });
    setDeployPolicyResult(policyRes.data ?? null);
    if (policyRes.data && !policyRes.data.passed) {
      setDeployLoading(false);
      toast('Policy check failed — review violations before deploying', 'error');
      return;
    }

    const res = await apiPost('/workloads', { spec_yaml: yaml });
    setDeployLoading(false);
    if (res.success) {
      const createdName =
        parseCreatedWorkloadName(typeof res.data === 'string' ? res.data : undefined) ??
        workloadNameFromYaml(yaml);
      markFirstDeploy();
      if (validateRes.data?.valid) {
        markSpecValidated();
      }
      toast(
        createdName ? `Deployed "${createdName}" successfully` : 'Workload deployed successfully',
        'success',
      );
      setSourceFilter('aether');
      void load();
      if (createdName) {
        setDeploySuccess({ name: createdName, status: 'pending' });
      } else {
        closeDeployModal();
      }
    } else {
      const err = res.error ?? 'unknown error';
      const friendly = err.includes('AlreadyExists') || err.includes('already exists')
        ? 'A workload with this name already exists in the cluster. Change metadata.name in the YAML and try again.'
        : err;
      toast(`Deploy failed: ${friendly}`, 'error');
    }
  }

  function toggleSelect(name: string) {
    setSelectedNames((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }

  async function bulkAction(action: 'start' | 'stop') {
    const names = [...selectedNames].filter((n) => {
      const w = workloads.find((x) => x.name === n);
      return w && isAetherManaged(w);
    });
    if (names.length === 0) return;
    setActionLoading(`bulk-${action}`);
    for (const name of names) {
      await apiPost(`/workloads/${name}/${action}`);
    }
    setActionLoading(null);
    setSelectedNames(new Set());
    toast(`Bulk ${action} completed for ${names.length} workload(s)`, 'success');
    load();
  }

  async function bulkDelete() {
    const names = [...selectedNames].filter((n) => {
      const w = workloads.find((x) => x.name === n);
      return w && isAetherManaged(w);
    });
    if (names.length === 0) return;
    setActionLoading('bulk-delete');
    for (const name of names) {
      await apiDelete(`/workloads/${name}`, { label: `Delete workload "${name}"` });
    }
    setActionLoading(null);
    setSelectedNames(new Set());
    setBulkConfirmDelete(false);
    toast(`Deleted ${names.length} workload(s)`, 'success');
    load();
  }

  async function handleUpdate(name: string, yaml: string) {
    setUpdateLoading(true);
    const res = await apiPut(`/workloads/${name}`, { yaml });
    setUpdateLoading(false);
    if (res.success) {
      toast(`Workload "${name}" updated`, 'success');
      setUpdateModal(null);
      load();
    } else {
      toast(res.error ?? 'Update failed', 'error');
    }
  }

  function handleConfirmedAction() {
    if (!confirmAction) return;
    handleAction(confirmAction.name, confirmAction.type);
    setConfirmAction(null);
  }

  if (loading && workloads.length === 0 && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="Workloads unavailable" onRetry={() => void load()} />;
  }

  const kinds = ['all', ...Array.from(new Set(workloads.map((w) => w.kind).filter((kind): kind is string => Boolean(kind)))).sort()];
  const clusters = ['all', ...Array.from(new Set(workloads.map((w) => w.cluster).filter((cluster): cluster is string => Boolean(cluster)))).sort()];
  const namespaces = ['all', ...Array.from(new Set(workloads.map((w) => w.namespace).filter((namespace): namespace is string => Boolean(namespace)))).sort()];

  const sourceFilterVal = sourceFilter as 'all' | 'aether' | 'cluster';

  const filteredWorkloads = sortWorkloadsForDisplay(
    workloads.filter((workload) => {
    const matchesSearch = !search || [
      workload.name,
      workload.image,
      workload.runtime,
      workload.cluster ?? '',
      workload.namespace ?? '',
      workload.kind ?? '',
    ].some((value) => value.toLowerCase().includes(search.toLowerCase()));
    const source = workload.source ?? 'aether';
    const matchesSource = sourceFilterVal === 'all' || source === sourceFilterVal;
    const matchesKind = kindFilter === 'all' || workload.kind === kindFilter;
    const matchesCluster = clusterFilter === 'all' || workload.cluster === clusterFilter;
    const matchesNamespace = namespaceFilter === 'all' || workload.namespace === namespaceFilter;
    const status = workload.status.toLowerCase();
    const isRunning = status === 'running' || status.includes('deployed');
    const isStopped = ['stopped', 'exited'].includes(status);
    const matchesStatus =
      statusFilter === 'all' ||
      (statusFilter === 'running' && isRunning) ||
      (statusFilter === 'stopped' && isStopped);
    const matchesPinned = !pinnedOnly || pinnedNames.includes(workload.name);
    return matchesSearch && matchesSource && matchesKind && matchesCluster && matchesNamespace && matchesStatus && matchesPinned;
    }),
  );
  const pinnedSet = new Set(pinnedNames);
  const displayWorkloads = [...filteredWorkloads].sort((a, b) => {
    const ap = pinnedSet.has(a.name) ? 0 : 1;
    const bp = pinnedSet.has(b.name) ? 0 : 1;
    return ap - bp;
  });

  function exportFilteredCsv() {
    const csv = workloadsToCsv(
      displayWorkloads.map((w) => ({
        name: w.name,
        kind: w.kind,
        runtime: w.runtime,
        image: w.image,
        status: w.status,
        cluster: w.cluster,
        namespace: w.namespace,
        source: w.source,
        created_at: w.created_at,
      })),
    );
    const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    downloadTextFile(`aether-workloads-${stamp}.csv`, csv);
    toast(`Exported ${displayWorkloads.length} workload(s)`, 'success');
  }

  const aetherManagedCount = countAetherManaged(workloads);
  const clusterDiscoveredCount = workloads.length - aetherManagedCount;
  const runningCount = workloads.filter((w) => {
    const s = w.status.toLowerCase();
    return s === 'running' || s.includes('deployed');
  }).length;
  const stoppedCount = workloads.filter((w) => ['stopped', 'exited'].includes(w.status.toLowerCase())).length;

  const filterSelectClass = 'glass-select';
  // .glass-select applies width:100% and wins the cascade over w-* utility classes in
  // this build's stylesheet ordering, so override width inline instead.
  const filterSelectStyle = { width: 'auto', minWidth: '9rem' } as const;

  const workloadColumns: DataTableColumn<WorkloadResponse>[] = [
    {
      key: 'select',
      header: '',
      width: 28,
      render: (w) =>
        isAetherManaged(w) ? (
          <input
            type="checkbox"
            checked={selectedNames.has(w.name)}
            onChange={(e) => {
              e.stopPropagation();
              toggleSelect(w.name);
            }}
            onClick={(e) => e.stopPropagation()}
            aria-label={`Select ${w.name}`}
          />
        ) : null,
    },
    {
      key: 'name',
      header: 'Name',
      sortValue: (w) => w.name,
      render: (w) => {
        const shortName = clusterResourceName(w.name);
        const locationLabel = [w.cluster, w.namespace].filter(Boolean).join('/');
        const metaLabel = [w.kind, locationLabel || null].filter(Boolean).join(' · ');
        const pinned = pinnedSet.has(w.name);
        return (
          <div className="flex min-w-0 items-start gap-1.5">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                togglePin(w.name);
              }}
              className={`mt-0.5 shrink-0 transition-colors ${pinned ? 'text-primary' : 'text-subtle hover:text-muted'}`}
              title={pinned ? 'Unpin workload' : 'Pin workload'}
            >
              <Star size={13} className={pinned ? 'fill-current' : ''} />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                openWorkloadDetail(w, 'overview');
              }}
              className="block w-full min-w-0 truncate text-left text-foreground hover:text-primary"
              title={w.name}
            >
              <span className="block truncate">{shortName}</span>
              {metaLabel ? (
                <span className="mt-0.5 block truncate text-[11px] font-normal text-subtle" title={metaLabel}>
                  {metaLabel}
                </span>
              ) : null}
            </button>
          </div>
        );
      },
    },
    {
      key: 'runtime',
      header: 'Runtime',
      width: 110,
      render: (w) => <RuntimeBadge runtime={w.runtime} />,
    },
    {
      key: 'image',
      header: 'Image',
      width: 220,
      render: (w) => (
        <code className="block truncate rounded bg-surface px-2 py-1 text-xs text-muted" title={w.image}>
          {w.image}
        </code>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: 110,
      sortValue: (w) => w.status,
      render: (w) => <Badge text={w.status} variant={getStatusVariant(w.status)} />,
    },
    {
      key: 'actions',
      header: 'Actions',
      width: 140,
      render: (w) => {
        const discovered = !isAetherManaged(w);
        const showLogs = !discovered || hasClusterLogs(w.kind);
        const showShell = discovered ? canMutate && isShellableClusterKind(w.kind) : canMutate;
        const showShellDenied = discovered ? !canMutate && isShellableClusterKind(w.kind) : !canMutate;
        const stop = (e: MouseEvent) => e.stopPropagation();
        return (
          <div className="flex flex-nowrap items-center gap-0.5" onClick={stop}>
            {discovered ? (
              <>
                <button type="button" onClick={() => openWorkloadDetail(w, 'overview')} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Info"><Info size={14} /></button>
                {showLogs ? (
                  <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Logs"><FileText size={14} /></button>
                ) : null}
                {showShell ? (
                  <button type="button" onClick={() => openWorkloadDetail(w, 'overview', true)} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Exec into pod"><Terminal size={14} /></button>
                ) : showShellDenied ? (
                  <button type="button" disabled className="cursor-not-allowed rounded p-1.5 text-subtle opacity-40" title="Exec requires Operator or Admin role"><Terminal size={14} /></button>
                ) : null}
              </>
            ) : (
              <>
                <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Logs"><FileText size={14} /></button>
                {canMutate ? (
                  <>
                    <button type="button" onClick={() => handleAction(w.name, 'start')} disabled={actionLoading === `${w.name}-start`} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Start"><Play size={14} /></button>
                    <button type="button" onClick={() => setConfirmAction({ type: 'stop', name: w.name })} disabled={actionLoading === `${w.name}-stop`} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-foreground" title="Stop"><Square size={14} /></button>
                    <button type="button" onClick={() => setConfirmAction({ type: 'delete', name: w.name })} disabled={actionLoading === `${w.name}-delete`} className="rounded p-1.5 text-subtle hover:bg-hover hover:text-danger" title="Delete"><Trash2 size={14} /></button>
                  </>
                ) : null}
              </>
            )}
          </div>
        );
      },
    },
  ];

  return (
    <AuroraPage view="workloads">
    <div className="overflow-x-hidden space-y-10">
      <StatRibbon
        testId="workloads-stats"
        items={[
          { label: 'Total', value: workloads.length },
          { label: 'Running', value: runningCount, tone: 'emerald' },
          { label: 'Aether', value: aetherManagedCount, tone: 'sky' },
          { label: 'Discovered', value: clusterDiscoveredCount },
        ]}
      />
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search name, image, cluster, namespace..."
        searchTestId="workloads-search"
        refreshTestId="workloads-refresh"
        onRefresh={load}
        refreshing={loading}
        filters={
          <>
            <select value={sourceFilterVal} onChange={(e) => setSourceFilter(e.target.value)} className={filterSelectClass} style={filterSelectStyle} data-testid="workloads-source-filter">
              <option value="all">All sources</option>
              <option value="aether">Aether managed</option>
              <option value="cluster">Kubernetes discovered</option>
            </select>
            <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className={filterSelectClass} style={filterSelectStyle} data-testid="workloads-status-filter">
              <option value="all">All statuses</option>
              <option value="running">Running</option>
              <option value="stopped">Stopped</option>
            </select>
          </>
        }
        actions={
          <>
            <button
              type="button"
              data-testid="workloads-export-csv"
              onClick={exportFilteredCsv}
              className="btn-secondary inline-flex items-center gap-2"
              title="Export filtered workloads as CSV"
            >
              <Download size={16} />
              Export CSV
            </button>
            {canMutate ? (
          <>
            <button
              type="button"
              data-testid="workloads-deploy-button"
              onClick={() => openDeployModal(setDeployYaml, setDeployModal)}
              className="btn-primary inline-flex items-center gap-2"
            >
              <Plus size={16} />
              Deploy
            </button>
            <button
              type="button"
              data-testid="workloads-validate-button"
              onClick={() => setValidateModal(true)}
              className="btn-secondary inline-flex items-center gap-2"
            >
              <ClipboardCheck size={16} />
              Validate
            </button>
          </>
          ) : null}
          </>
        }
      />

      <div className="flex items-start gap-4">
        <div
          className={selectedWorkload ? 'hidden min-w-0 flex-1 lg:block' : 'min-w-0 flex-1'}
          data-testid="workloads-list-column"
        >
      <details className="mb-4" data-testid="workloads-more-filters">
        <summary className="cursor-pointer text-sm text-muted hover:text-foreground">More filters</summary>
        <div className="mt-3 space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap gap-1.5" data-testid="workloads-kind-chips">
          {(kinds.length > 2 ? kinds : ['all']).map((kind) => {
            const count = kind === 'all'
              ? workloads.length
              : workloads.filter((w) => w.kind === kind).length;
            const active = kindFilter === kind;
            return (
              <button
                key={kind}
                type="button"
                onClick={() => setKindFilter(kind)}
                className={`rounded-full px-2.5 py-1 text-[11px] transition-colors ${
                  active
                    ? 'bg-primary/20 text-primary border border-primary/40'
                    : 'glass-inset-surface text-muted border glass-divider hover:text-foreground'
                }`}
              >
                {kind === 'all' ? 'All' : kind}
                <span className="ml-1 opacity-70">{count}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-2" data-testid="workloads-status-chips">
        <SegmentedControl
          value={statusFilter as 'all' | 'running' | 'stopped'}
          onChange={(id) => setStatusFilter(id)}
          items={[
            { key: 'all', label: 'All', count: workloads.length },
            { key: 'running', label: 'Running', count: runningCount },
            { key: 'stopped', label: 'Stopped', count: stoppedCount },
          ]}
        />
        {namespaces.length > 2
          ? namespaces.slice(0, 10).filter((n) => n !== 'all').map((namespace) => {
              const count = workloads.filter((w) => w.namespace === namespace).length;
              const active = namespaceFilter === namespace;
              return (
                <button
                  key={namespace}
                  type="button"
                  onClick={() => setNamespaceFilter(active ? 'all' : namespace)}
                  className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] transition-colors ${
                    active
                      ? 'border-primary-wash bg-primary-wash text-primary'
                      : 'border-rule text-subtle hover:text-muted'
                  }`}
                >
                  <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${categoricalDotClass(namespace)}`} aria-hidden />
                  {namespace}
                  <span className="opacity-70">{count}</span>
                </button>
              );
            })
          : null}
        <select value={kindFilter} onChange={(e) => setKindFilter(e.target.value)} className={filterSelectClass} style={filterSelectStyle} data-testid="workloads-kind-filter">
          {kinds.map((kind) => <option key={kind} value={kind}>{kind === 'all' ? 'All kinds' : kind}</option>)}
        </select>
        <select value={clusterFilter} onChange={(e) => setClusterFilter(e.target.value)} className={filterSelectClass} style={filterSelectStyle} data-testid="workloads-cluster-filter">
          {clusters.map((cluster) => <option key={cluster} value={cluster}>{cluster === 'all' ? 'All clusters' : cluster}</option>)}
        </select>
        <select value={namespaceFilter} onChange={(e) => setNamespaceFilter(e.target.value)} className={filterSelectClass} style={filterSelectStyle} data-testid="workloads-namespace-filter">
          {namespaces.map((namespace) => <option key={namespace} value={namespace}>{namespace === 'all' ? 'All namespaces' : namespace}</option>)}
        </select>
        <button
          type="button"
          onClick={() => setPinnedOnly((v) => !v)}
          className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm transition-colors ${
            pinnedOnly
              ? 'bg-primary/20 text-primary border border-primary/40'
              : 'glass-inset-surface text-muted border glass-divider hover:text-foreground'
          }`}
          title="Show pinned workloads only"
          data-testid="workloads-pinned-filter"
        >
          <Star size={14} className={pinnedOnly ? 'fill-current' : ''} />
          Pinned{pinnedNames.length > 0 ? ` (${pinnedNames.length})` : ''}
        </button>
      </div>
        </div>
      </details>

      <div className="mb-4 flex items-center justify-end gap-1 rounded-xl border glass-divider p-1 w-fit ml-auto" data-testid="workloads-view-toggle">
          <button
            type="button"
            onClick={() => setViewModePersist('cards')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] ${viewMode === 'cards' ? 'bg-primary/20 text-primary' : 'text-muted hover:text-foreground'}`}
            title="Card view"
          >
            <LayoutGrid size={14} />
            Cards
          </button>
          <button
            type="button"
            onClick={() => setViewModePersist('table')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] ${viewMode === 'table' ? 'bg-primary/20 text-primary' : 'text-muted hover:text-foreground'}`}
            title="Table view"
          >
            <List size={14} />
            Table
          </button>
      </div>

      {workloadParam.trim() && !selectedWorkload ? (
        <WorkloadContextBanner
          testId="workloads-context-banner"
          workload={workloadParam.trim()}
          description="Workload list context"
        >
          <WorkloadScopedCrossLinks
            workload={workloadParam.trim()}
            prefix="workloads"
            showDrift
            showGitops
            showMetrics
            extraLinks={[
              { label: 'Platform →', view: 'platform', testId: 'workloads-platform-link', query: { workload: workloadParam.trim() } },
              { label: 'Clusters →', view: 'clusters', testId: 'workloads-clusters-link', query: { workload: workloadParam.trim() } },
              { label: 'OpenAPI →', view: 'openapi', testId: 'workloads-openapi-link', query: { workload: workloadParam.trim() } },
              { label: 'RBAC →', view: 'rbac', testId: 'workloads-rbac-link', query: { workload: workloadParam.trim() } },
              { label: 'Policy →', view: 'policy', testId: 'workloads-context-policy-link', query: { workload: workloadParam.trim() } },
              { label: 'Secrets →', view: 'secrets', testId: 'workloads-context-secrets-link', query: { workload: workloadParam.trim() } },
              { label: 'Editor →', view: 'editor', testId: 'workloads-context-editor-link', query: { workload: workloadParam.trim() } },
              { label: 'Copilot →', view: 'zyra', testId: 'workloads-context-copilot-link', query: { workload: workloadParam.trim(), q: `Summarize ${workloadParam.trim()}` } },
              { label: 'Compose →', view: 'compose', testId: 'workloads-context-compose-link', query: { workload: workloadParam.trim() } },
              { label: 'Dependencies →', view: 'deps', testId: 'workloads-context-deps-link', query: { workload: workloadParam.trim() } },
            ]}
          />
        </WorkloadContextBanner>
      ) : null}

      {sourceFilterVal !== 'all' ? (
        <div className="glass-context-banner mb-4 text-sm text-muted">
          Showing <span className="text-foreground">{filteredWorkloads.length}</span> of{' '}
          <span className="text-foreground">{workloads.length}</span> workloads
          {sourceFilterVal === 'aether' ? ' (Aether managed only)' : ' (Kubernetes discovered only)'}.
          {sourceFilterVal === 'aether' && clusterDiscoveredCount > 0 ? (
            <>
              {' '}
              <button type="button" onClick={() => setSourceFilter('cluster')} className="text-primary hover:underline">
                View {clusterDiscoveredCount} discovered
              </button>
            </>
          ) : null}
          {sourceFilterVal === 'cluster' && aetherManagedCount > 0 ? (
            <>
              {' '}
              <button type="button" onClick={() => setSourceFilter('aether')} className="text-primary hover:underline">
                View {aetherManagedCount} Aether-managed
              </button>
            </>
          ) : null}
          {' · '}
          <button type="button" onClick={() => setSourceFilter('all')} className="text-primary hover:underline">
            show all
          </button>
        </div>
      ) : null}

      <div className="mb-8 flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted" data-testid="workloads-filter-stats">
        <button type="button" onClick={() => { setStatusFilter('all'); setSourceFilter('all'); }} className="hover:text-foreground">
          All
        </button>
        <button type="button" data-testid="workloads-running-stat" onClick={() => setStatusFilter('running')} className="hover:text-foreground">
          Running
        </button>
        <button type="button" onClick={() => setStatusFilter('stopped')} className="hover:text-foreground">
          Stopped
        </button>
        <button type="button" data-testid="workloads-aether-stat" onClick={() => setSourceFilter('aether')} className="hover:text-foreground">
          Aether-managed
        </button>
        <button type="button" data-testid="workloads-discovered-stat" onClick={() => setSourceFilter('cluster')} className="hover:text-foreground">
          Discovered
        </button>
      </div>

      {selectedNames.size > 0 && canMutate && (
        <div className="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-primary/30 bg-primary/5 px-4 py-3" data-testid="workloads-bulk-bar">
          <span className="text-sm text-muted">{selectedNames.size} selected</span>
          <button type="button" onClick={() => void bulkAction('start')} className="px-3 py-1.5 text-sm rounded-lg bg-success text-white hover:bg-success">Start all</button>
          <button type="button" onClick={() => void bulkAction('stop')} className="px-3 py-1.5 text-sm rounded-lg bg-warning text-white hover:bg-warning">Stop all</button>
          <button type="button" onClick={() => setBulkConfirmDelete(true)} className="px-3 py-1.5 text-sm rounded-lg bg-danger text-white hover:bg-danger">Delete all</button>
          <button type="button" onClick={() => setSelectedNames(new Set())} className="px-3 py-1.5 text-sm text-muted hover:text-foreground">Clear</button>
        </div>
      )}

      {filteredWorkloads.length === 0 ? (
        workloads.length === 0 ? (
          <EmptyState
            icon={<Rocket size={48} />}
            title="No workloads yet"
            description="Deploy a YAML spec, design one in the visual editor, or import Docker Compose to get started."
            action={
              canMutate ? (
              <div className="flex flex-wrap justify-center gap-2">
                <button
                  type="button"
                  onClick={() => openDeployModal(setDeployYaml, setDeployModal)}
                  className="btn-primary inline-flex items-center gap-2"
                >
                  <Plus className="h-4 w-4" />
                  Deploy YAML
                </button>
                <button
                  type="button"
                  onClick={() => navigate(viewToPath('editor'))}
                  className="btn-secondary inline-flex items-center gap-2"
                >
                  <FileCode2 className="h-4 w-4" />
                  Visual Editor
                </button>
                <button
                  type="button"
                  onClick={() => setValidateModal(true)}
                  className="btn-secondary inline-flex items-center gap-2"
                  data-testid="workloads-empty-validate-button"
                >
                  <ClipboardCheck className="h-4 w-4" />
                  Validate YAML
                </button>
                <button
                  type="button"
                  onClick={() => navigate(viewToPath('compose'))}
                  className="btn-secondary inline-flex items-center gap-2"
                >
                  <Layers className="h-4 w-4" />
                  Import Compose
                </button>
              </div>
              ) : null
            }
          />
        ) : (
          <EmptyState icon={<Inbox size={48} />} title="No matching workloads" description="Try adjusting your search or filters" />
        )
      ) : viewMode === 'cards' ? (
        <div
          className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4"
          data-testid="workloads-card-grid"
        >
          {displayWorkloads.map((w, i) => (
            <WorkloadCard
              key={w.name}
              workload={w}
              pinned={pinnedSet.has(w.name)}
              canMutate={canMutate}
              index={i}
              onOpen={(tab = 'overview', openShell = false) => openWorkloadDetail(w, tab, openShell)}
              onTogglePin={() => togglePin(w.name)}
            />
          ))}
        </div>
      ) : (
        <div data-testid="workloads-table">
          {canMutate && filteredWorkloads.some((w) => isAetherManaged(w)) ? (
            <div className="mb-2 flex items-center gap-2 text-[11.5px] text-subtle">
              <input
                type="checkbox"
                aria-label="Select all"
                checked={filteredWorkloads.filter((w) => isAetherManaged(w)).every((w) => selectedNames.has(w.name))}
                onChange={(e) => {
                  const managed = filteredWorkloads.filter((w) => isAetherManaged(w)).map((w) => w.name);
                  setSelectedNames(e.target.checked ? new Set(managed) : new Set());
                }}
              />
              Select all
            </div>
          ) : null}
          <DataTable<WorkloadResponse>
            items={displayWorkloads}
            getId={(w) => w.name}
            getStatus={(w) => w.status}
            sortBySeverityDefault={false}
            emptyTitle="No matching workloads"
            emptyBody="Try adjusting your search or filters."
            columns={workloadColumns}
          />
        </div>
      )}
        </div>

        {selectedWorkload ? (
          <aside
            ref={detailRef}
            className="animate-ac-slide w-full shrink-0 scroll-mt-4 lg:w-[45%] lg:min-w-[430px]"
            data-testid="workloads-detail-anchor"
          >
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2 lg:hidden">
              <button
                type="button"
                onClick={closeWorkloadDetail}
                className="btn-secondary !px-3 !py-1.5 !text-xs"
              >
                ← Back to inventory
              </button>
            </div>
            <WorkloadDetail
              key={`${selectedWorkload.name}-${detailInitialTab}-${detailInitialShell ? 'shell' : 'noshell'}`}
              workload={selectedWorkload}
              initialTab={detailInitialTab}
              initialShellOpen={detailInitialShell}
              canMutate={canMutate}
              onClose={closeWorkloadDetail}
              onMigrate={isAetherManaged(selectedWorkload) ? (name) => setMigrateModal(name) : undefined}
              onAction={() => load()}
            />
          </aside>
        ) : null}
      </div>

      <Modal
        isOpen={bulkConfirmDelete}
        onClose={() => setBulkConfirmDelete(false)}
        title="Confirm bulk delete"
      >
        <p className="text-sm text-muted mb-6">
          Delete {selectedNames.size} selected workload(s)? This cannot be undone.
        </p>
        <div className="flex justify-end gap-3">
          <button type="button" onClick={() => setBulkConfirmDelete(false)} className="btn-secondary">
            Cancel
          </button>
          <button type="button" onClick={() => void bulkDelete()} className="btn-danger">
            Delete all
          </button>
        </div>
      </Modal>

      <Modal
        isOpen={confirmAction !== null}
        onClose={() => setConfirmAction(null)}
        title={confirmAction?.type === 'delete' ? 'Confirm Delete' : 'Confirm Stop'}
      >
        <p className="text-sm text-muted mb-6">
          {confirmAction?.type === 'delete'
            ? `Are you sure you want to delete "${confirmAction.name}"?`
            : `Stop workload "${confirmAction?.name}"?`}
        </p>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setConfirmAction(null)}
            className="btn-secondary"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleConfirmedAction}
            className={confirmAction?.type === 'delete' ? 'btn-danger' : 'btn-primary'}
          >
            {confirmAction?.type === 'delete' ? 'Delete' : 'Stop'}
          </button>
        </div>
      </Modal>

      <Modal
        isOpen={migrateModal !== null}
        onClose={() => {
          setMigrateModal(null);
          setMigrateAdvice(null);
          setMigrateTarget(null);
          setMigrateStrategy('blue-green');
          setMigrateResult(null);
        }}
        title={`Migrate: ${migrateModal}`}
        size="wide"
      >
        <div data-testid="workloads-migrate-modal">
        <p className="text-sm text-muted mb-4">Select target runtime to load migration advice:</p>
        <div className="grid grid-cols-2 gap-3 mb-6">
          {runtimes.map((rt) => (
            <button
              key={rt}
              type="button"
              onClick={() => migrateModal && void loadMigrationAdvice(migrateModal, rt)}
              disabled={adviceLoading || actionLoading !== null}
              className={`px-4 py-3 border rounded-lg text-sm font-medium transition-colors capitalize ${
                migrateTarget === rt
                  ? 'border-primary/50 bg-primary/10 text-primary'
                  : 'glass-inset-surface glass-inset-hover glass-divider text-foreground'
              }`}
            >
              {adviceLoading && migrateTarget === rt ? 'Loading advice…' : rt}
            </button>
          ))}
        </div>

        {migrateAdvice && migrateTarget && (
          <div className="glass p-4 space-y-3">
            <h4 className="text-sm font-semibold text-foreground">
              Advice: {migrateAdvice.source_runtime} → {migrateAdvice.target_runtime}
            </h4>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-subtle">Strategy</span>
                <p className="text-foreground">{migrateAdvice.recommended_strategy}</p>
              </div>
              <div>
                <span className="text-subtle">Risk</span>
                <p className="text-foreground">{migrateAdvice.risk_level}</p>
              </div>
              <div>
                <span className="text-subtle">Est. downtime</span>
                <p className="text-foreground">{migrateAdvice.estimated_downtime_secs}s</p>
              </div>
              {migrateAdvice.timing && (
                <div>
                  <span className="text-subtle">Timing</span>
                  <p className="text-foreground">{migrateAdvice.timing.recommendation}</p>
                </div>
              )}
            </div>
            <div>
              <span className="text-xs uppercase tracking-wider text-subtle mb-2 block">Migration strategy</span>
              <div className="flex flex-wrap gap-2">
                {(['immediate', 'blue-green', 'rolling'] as const).map((s) => (
                  <label key={s} className="inline-flex items-center gap-2 text-sm text-muted cursor-pointer">
                    <input
                      type="radio"
                      name="migrate-strategy"
                      value={s}
                      checked={migrateStrategy === s}
                      onChange={() => setMigrateStrategy(s)}
                      className="accent-primary"
                    />
                    {s}
                  </label>
                ))}
              </div>
            </div>
            {migrateAdvice.reasons.length > 0 && (
              <ul className="text-xs text-muted space-y-1">
                {migrateAdvice.reasons.map((r, i) => (
                  <li key={i}>+ {r}</li>
                ))}
              </ul>
            )}
            {migrateAdvice.warnings.length > 0 && (
              <ul className="text-xs text-warning/90 space-y-1">
                {migrateAdvice.warnings.map((w, i) => (
                  <li key={i}>! {w}</li>
                ))}
              </ul>
            )}
            <button
              type="button"
              onClick={() => migrateModal && migrateTarget && handleMigrate(migrateModal, migrateTarget, migrateStrategy)}
              disabled={actionLoading !== null}
              className="w-full py-3 bg-primary hover:bg-aether-light disabled:opacity-50 rounded-xl font-medium text-[var(--primary-foreground)]"
            >
              {actionLoading ? 'Starting migration…' : `Start migration to ${migrateTarget}`}
            </button>
            {migrateResult ? (
              <p
                data-testid="migrate-result"
                className={`text-sm rounded-lg border px-3 py-2 ${
                  migrateResult.toLowerCase().includes('failed')
                    ? 'border-danger/30 bg-danger/5 text-danger'
                    : 'border-success/30 bg-success/5 text-success'
                }`}
              >
                {migrateResult}
              </p>
            ) : null}
          </div>
        )}
        </div>
      </Modal>

      <Modal isOpen={updateModal !== null} onClose={() => setUpdateModal(null)} title={`Update: ${updateModal}`} size="yaml">
        <div data-testid="workloads-update-modal">
        <YamlInput
          layout="editor"
          buttonText="Apply update"
          onSubmit={(yaml) => updateModal && handleUpdate(updateModal, yaml)}
          loading={updateLoading}
          submitTestId="workloads-update-submit"
          placeholder="Paste updated workload YAML (metadata.name must match)..."
        />
        </div>
      </Modal>

      <Modal isOpen={deployModal} onClose={closeDeployModal} title="Deploy New Workload" size="full">
        {deploySuccess ? (
          <DeploySuccessPanel
            name={deploySuccess.name}
            status={deploySuccess.status}
            onViewLogs={() => finishDeploySuccess('logs')}
            onClose={() => finishDeploySuccess('overview')}
          />
        ) : (
          <div data-testid="workloads-deploy-modal">
          <YamlInput
            value={deployYaml}
            onChange={setDeployYaml}
            resetValue={DEFAULT_DEPLOY_WORKLOAD_YAML}
            layout="editor"
            buttonText="Deploy"
            onSubmit={handleDeploy}
            loading={deployLoading}
            editorLoading={deployTemplateLoading}
            showValidateButton
            onValidate={handleDeployValidate}
            validateLoading={deployInlineValidateLoading}
            submitTestId="workloads-deploy-submit"
            placeholder="Paste aether/v1 Workload YAML (see examples/ in the repo)..."
            footer={
              <ValidateResultPanel validate={deployValidateResult} policy={deployPolicyResult} />
            }
          />
          </div>
        )}
      </Modal>

      <Modal isOpen={validateModal} onClose={() => { setValidateModal(false); setValidateResult(null); }} title="Validate Workload YAML" size="yaml">
        <div data-testid="workloads-validate-modal">
        <YamlInput
          key="validate-default"
          initialValue={DEFAULT_DEPLOY_WORKLOAD_YAML}
          resetValue={DEFAULT_DEPLOY_WORKLOAD_YAML}
          layout="editor"
          buttonText="Validate"
          onSubmit={handleValidate}
          loading={validateLoading}
          submitTestId="workloads-validate-submit"
          placeholder="Paste aether/v1 Workload YAML (see examples/ in the repo)..."
          footer={<ValidateResultPanel validate={validateResult} />}
        />
        </div>
      </Modal>
    </div>
    </AuroraPage>
  );
}

export default WorkloadsPage;