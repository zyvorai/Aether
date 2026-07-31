// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router';
import { Play, Square, Trash2, FileText, ClipboardCheck, Inbox, Plus, Rocket, FileCode2, Layers, Terminal, Info, Star, Download, LayoutGrid, List } from 'lucide-react';
import { apiFetch, apiFetchSettled, apiPost, apiDelete, apiPut } from '../../utils/api';
import { useQueryParam, pathWithQuery } from '../../utils/urlState';
import { viewToPath } from '../../utils/dashboardRoutes';
import { markSpecValidated, markFirstDeploy, syncDeployFromWorkloads } from '../../utils/onboardingState';
import { useAuth } from '../../contexts/AuthContext';
import { DEFAULT_DEPLOY_WORKLOAD_YAML, freshDeployWorkloadYaml, mergeConfidentialIntoYaml, workloadJsonToYaml, workloadNameFromYaml } from '../../utils/workloadYaml';
import ConfidentialFormFields, {
  defaultConfidentialFormState,
  type ConfidentialFormState,
} from '../ConfidentialFormFields';
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
import Modal from '../Modal';
import YamlInput from '../YamlInput';
import ValidateResultPanel from '../ValidateResultPanel';
import DeploySuccessPanel from '../DeploySuccessPanel';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, ValidateResponse, MigrationAdvice, PolicyResult, ConfidentialMigrationPlan } from '../../types/api';
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
  setDeployConfidential: (state: ConfidentialFormState) => void,
  setDeployModal: (open: boolean) => void,
) {
  setDeployYaml(freshDeployWorkloadYaml());
  setDeployConfidential(defaultConfidentialFormState);
  setDeployModal(true);
}

export default function WorkloadsPage({ initialSelectedName, onClearInitialSelection, refreshKey }: WorkloadsPageProps) {
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
  const [confidentialMigrationPlan, setConfidentialMigrationPlan] = useState<ConfidentialMigrationPlan | null>(null);
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
  const [deployConfidential, setDeployConfidential] = useState<ConfidentialFormState>(defaultConfidentialFormState);
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
      return localStorage.getItem('aether_workloads_view') === 'table' ? 'table' : 'cards';
    } catch {
      return 'cards';
    }
  });
  const detailRef = useRef<HTMLDivElement | null>(null);

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];

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
      setDeployConfidential(defaultConfidentialFormState);
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
    const tab = (['overview', 'logs', 'topology', 'manifest', 'drift', 'scoring', 'events', 'trust'] as const).includes(
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
    const [data, confPlan] = await Promise.all([
      apiFetch<MigrationAdvice>(`/ai/migration-advice/${encodeURIComponent(name)}/${encodeURIComponent(target)}`),
      apiFetch<ConfidentialMigrationPlan>(`/confidential/migration-plan/${encodeURIComponent(name)}/${encodeURIComponent(target)}`),
    ]);
    setMigrateAdvice(data);
    if (confPlan?.recommended_strategy?.includes('Confidential')) {
      setMigrateStrategy('confidential-blue-green');
    } else if (data?.recommended_strategy) {
      const s = data.recommended_strategy.toLowerCase().replace(/_/g, '-');
      if (s.includes('immediate')) setMigrateStrategy('immediate');
      else if (s.includes('rolling')) setMigrateStrategy('rolling');
      else if (s.includes('canary')) setMigrateStrategy('canary');
      else if (s.includes('confidential')) setMigrateStrategy('confidential-blue-green');
      else setMigrateStrategy('blue-green');
    }
    setConfidentialMigrationPlan(confPlan);
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
    setDeployConfidential(defaultConfidentialFormState);
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

  return (
    <div className="overflow-x-hidden">
      {!selectedWorkload ? (
        <section className="workloads-masthead mb-5 px-5 py-5 sm:px-6 sm:py-6" data-testid="workloads-masthead">
          <div className="relative z-[1] flex flex-wrap items-end justify-between gap-4">
            <div className="min-w-0">
              <div className="mb-2 inline-flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-aether">
                <span className="h-1.5 w-1.5 rounded-full bg-aether platform-pulse" />
                Runtime inventory
              </div>
              <h2 className="text-2xl font-semibold tracking-tight text-white sm:text-3xl">Workloads</h2>
              <p className="mt-1.5 max-w-xl text-sm text-slate-400">
                Cards-first ops deck — open Info, stream Logs, or Exec without leaving the grid.
              </p>
            </div>
            <div className="flex flex-wrap gap-2 text-[11px]">
              <span className="rounded-full border border-emerald-500/25 bg-emerald-500/10 px-3 py-1 text-emerald-300">
                {runningCount} running
              </span>
              <span className="rounded-full border border-sky-500/25 bg-sky-500/10 px-3 py-1 text-sky-300">
                {clusterDiscoveredCount} discovered
              </span>
              <span className="rounded-full border border-violet-500/25 bg-violet-500/10 px-3 py-1 text-violet-200">
                {aetherManagedCount} managed
              </span>
            </div>
          </div>
        </section>
      ) : null}

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
            <select value={sourceFilterVal} onChange={(e) => setSourceFilter(e.target.value)} className={filterSelectClass} data-testid="workloads-source-filter">
              <option value="all">All sources</option>
              <option value="aether">Aether managed</option>
              <option value="cluster">Kubernetes discovered</option>
            </select>
            <select value={kindFilter} onChange={(e) => setKindFilter(e.target.value)} className={filterSelectClass} data-testid="workloads-kind-filter">
              {kinds.map((kind) => <option key={kind} value={kind}>{kind === 'all' ? 'All kinds' : kind}</option>)}
            </select>
            <select value={clusterFilter} onChange={(e) => setClusterFilter(e.target.value)} className={filterSelectClass} data-testid="workloads-cluster-filter">
              {clusters.map((cluster) => <option key={cluster} value={cluster}>{cluster === 'all' ? 'All clusters' : cluster}</option>)}
            </select>
            <select value={namespaceFilter} onChange={(e) => setNamespaceFilter(e.target.value)} className={filterSelectClass} data-testid="workloads-namespace-filter">
              {namespaces.map((namespace) => <option key={namespace} value={namespace}>{namespace === 'all' ? 'All namespaces' : namespace}</option>)}
            </select>
            <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className={filterSelectClass} data-testid="workloads-status-filter">
              <option value="all">All statuses</option>
              <option value="running">Running</option>
              <option value="stopped">Stopped</option>
            </select>
            <button
              type="button"
              onClick={() => setPinnedOnly((v) => !v)}
              className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-2 text-sm transition-colors ${
                pinnedOnly
                  ? 'bg-aether/20 text-aether border border-aether/40'
                  : 'glass-inset-surface text-slate-400 border glass-divider hover:text-slate-200'
              }`}
              title="Show pinned workloads only"
              data-testid="workloads-pinned-filter"
            >
              <Star size={14} className={pinnedOnly ? 'fill-current' : ''} />
              Pinned{pinnedNames.length > 0 ? ` (${pinnedNames.length})` : ''}
            </button>
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
              onClick={() => openDeployModal(setDeployYaml, setDeployConfidential, setDeployModal)}
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

      {selectedWorkload ? (
        <div
          ref={detailRef}
          className="mb-4 scroll-mt-4 grid gap-4 lg:grid-cols-[minmax(16rem,18rem)_minmax(0,1fr)]"
          data-testid="workloads-detail-anchor"
        >
          <aside className="hidden max-h-[calc(100vh-7rem)] flex-col gap-2 overflow-y-auto rounded-2xl border glass-divider p-2 lg:flex" data-testid="workloads-side-rail">
            <div className="sticky top-0 z-[1] mb-1 flex items-center justify-between gap-2 rounded-xl glass-inset-surface px-2 py-2">
              <button
                type="button"
                onClick={closeWorkloadDetail}
                className="text-[11px] text-slate-400 hover:text-aether"
                data-testid="workloads-back-to-list"
              >
                ← Inventory
              </button>
              <span className="text-[10px] text-slate-600">{displayWorkloads.length}</span>
            </div>
            {displayWorkloads.map((w) => (
              <WorkloadCard
                key={w.name}
                workload={w}
                pinned={pinnedSet.has(w.name)}
                selected={w.name === selectedWorkload.name}
                compact
                canMutate={canMutate}
                onOpen={(tab = 'overview', openShell = false) => openWorkloadDetail(w, tab, openShell)}
                onTogglePin={() => togglePin(w.name)}
              />
            ))}
          </aside>
          <div className="min-w-0">
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
          </div>
        </div>
      ) : null}

      {!selectedWorkload ? (
        <>
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
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
                    ? 'bg-aether/20 text-aether border border-aether/40'
                    : 'glass-inset-surface text-slate-400 border glass-divider hover:text-slate-200'
                }`}
              >
                {kind === 'all' ? 'All' : kind}
                <span className="ml-1 opacity-70">{count}</span>
              </button>
            );
          })}
        </div>
        <div className="flex items-center gap-1 rounded-xl border glass-divider p-1" data-testid="workloads-view-toggle">
          <button
            type="button"
            onClick={() => setViewModePersist('cards')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] ${viewMode === 'cards' ? 'bg-aether/20 text-aether' : 'text-slate-400 hover:text-slate-200'}`}
            title="Card view"
          >
            <LayoutGrid size={14} />
            Cards
          </button>
          <button
            type="button"
            onClick={() => setViewModePersist('table')}
            className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] ${viewMode === 'table' ? 'bg-aether/20 text-aether' : 'text-slate-400 hover:text-slate-200'}`}
            title="Table view"
          >
            <List size={14} />
            Table
          </button>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap gap-1.5" data-testid="workloads-status-chips">
        {(
          [
            { id: 'all', label: 'All', count: workloads.length },
            { id: 'running', label: 'Running', count: runningCount },
            { id: 'stopped', label: 'Stopped', count: stoppedCount },
          ] as const
        ).map((chip) => {
          const active = statusFilter === chip.id;
          return (
            <button
              key={chip.id}
              type="button"
              onClick={() => setStatusFilter(chip.id)}
              className={`rounded-full px-2.5 py-1 text-[11px] transition-colors ${
                active
                  ? 'bg-aether/20 text-aether border border-aether/40'
                  : 'glass-inset-surface text-slate-400 border glass-divider hover:text-slate-200'
              }`}
            >
              {chip.label}
              <span className="ml-1 opacity-70">{chip.count}</span>
            </button>
          );
        })}
        {namespaces.length > 2
          ? namespaces.slice(0, 10).filter((n) => n !== 'all').map((namespace) => {
              const count = workloads.filter((w) => w.namespace === namespace).length;
              const active = namespaceFilter === namespace;
              return (
                <button
                  key={namespace}
                  type="button"
                  onClick={() => setNamespaceFilter(active ? 'all' : namespace)}
                  className={`rounded-full px-2.5 py-1 text-[11px] transition-colors ${
                    active
                      ? 'bg-aether/20 text-aether border border-aether/40'
                      : 'glass-inset-surface text-slate-500 border glass-divider hover:text-slate-200'
                  }`}
                >
                  {namespace}
                  <span className="ml-1 opacity-70">{count}</span>
                </button>
              );
            })
          : null}
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
          />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('platform'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-platform-link"
          >
            Platform →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('clusters'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-clusters-link"
          >
            Clusters →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('openapi'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-openapi-link"
          >
            OpenAPI →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('rbac'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-rbac-link"
          >
            RBAC →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('policy'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-policy-link"
          >
            Policy →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('secrets'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-secrets-link"
          >
            Secrets →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('editor'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-editor-link"
          >
            Editor →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zeus'), { workload: workloadParam.trim(), q: `Summarize ${workloadParam.trim()}` })}
            className="text-aether hover:underline"
            data-testid="workloads-context-copilot-link"
          >
            Copilot →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('compose'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-compose-link"
          >
            Compose →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('deps'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-deps-link"
          >
            Dependencies →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('gitops'), { workload: workloadParam.trim() })}
            className="text-aether hover:underline"
            data-testid="workloads-context-gitops-link"
          >
            GitOps →
          </Link>
        </WorkloadContextBanner>
      ) : null}

      {sourceFilterVal !== 'all' ? (
        <div className="glass-context-banner mb-4 text-sm text-slate-400">
          Showing <span className="text-slate-200">{filteredWorkloads.length}</span> of{' '}
          <span className="text-slate-200">{workloads.length}</span> workloads
          {sourceFilterVal === 'aether' ? ' (Aether managed only)' : ' (Kubernetes discovered only)'}.
          {sourceFilterVal === 'aether' && clusterDiscoveredCount > 0 ? (
            <>
              {' '}
              <button type="button" onClick={() => setSourceFilter('cluster')} className="text-aether hover:underline">
                View {clusterDiscoveredCount} discovered
              </button>
            </>
          ) : null}
          {sourceFilterVal === 'cluster' && aetherManagedCount > 0 ? (
            <>
              {' '}
              <button type="button" onClick={() => setSourceFilter('aether')} className="text-aether hover:underline">
                View {aetherManagedCount} Aether-managed
              </button>
            </>
          ) : null}
          {' · '}
          <button type="button" onClick={() => setSourceFilter('all')} className="text-aether hover:underline">
            show all
          </button>
        </div>
      ) : null}

      <section className="mb-5 grid grid-cols-2 gap-px overflow-hidden rounded-2xl border glass-divider sm:grid-cols-3 xl:grid-cols-6" data-testid="workloads-stats">
        <button type="button" onClick={() => { setStatusFilter('all'); setSourceFilter('all'); }} className="glass-inset-surface px-3 py-3 text-left transition hover:bg-white/[0.03]">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Total</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-white">{workloads.length}</div>
        </button>
        <button type="button" data-testid="workloads-running-stat" onClick={() => setStatusFilter('running')} className="glass-inset-surface px-3 py-3 text-left transition hover:bg-emerald-500/5">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Running</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-emerald-300">{runningCount}</div>
        </button>
        <button type="button" onClick={() => setStatusFilter('stopped')} className="glass-inset-surface px-3 py-3 text-left transition hover:bg-amber-500/5">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Stopped</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-amber-200">{stoppedCount}</div>
        </button>
        <button type="button" data-testid="workloads-aether-stat" onClick={() => setSourceFilter('aether')} className="glass-inset-surface px-3 py-3 text-left transition hover:bg-violet-500/5">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Aether</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-violet-200">{aetherManagedCount}</div>
        </button>
        <button type="button" data-testid="workloads-discovered-stat" onClick={() => setSourceFilter('cluster')} className="glass-inset-surface px-3 py-3 text-left transition hover:bg-sky-500/5">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Discovered</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-sky-200">{clusterDiscoveredCount}</div>
        </button>
        <div className="glass-inset-surface px-3 py-3">
          <div className="text-[10px] uppercase tracking-wider text-slate-500">Namespaces</div>
          <div className="mt-0.5 text-xl font-semibold tabular-nums text-white">{namespaces.filter((namespace) => namespace !== 'all').length}</div>
        </div>
      </section>

      {selectedNames.size > 0 && canMutate && (
        <div className="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-aether/30 bg-aether/5 px-4 py-3" data-testid="workloads-bulk-bar">
          <span className="text-sm text-slate-300">{selectedNames.size} selected</span>
          <button type="button" onClick={() => void bulkAction('start')} className="px-3 py-1.5 text-sm rounded-lg bg-emerald-600 text-white hover:bg-emerald-500">Start all</button>
          <button type="button" onClick={() => void bulkAction('stop')} className="px-3 py-1.5 text-sm rounded-lg bg-amber-600 text-white hover:bg-amber-500">Stop all</button>
          <button type="button" onClick={() => setBulkConfirmDelete(true)} className="px-3 py-1.5 text-sm rounded-lg bg-red-600 text-white hover:bg-red-500">Delete all</button>
          <button type="button" onClick={() => setSelectedNames(new Set())} className="px-3 py-1.5 text-sm text-slate-400 hover:text-slate-200">Clear</button>
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
                  onClick={() => openDeployModal(setDeployYaml, setDeployConfidential, setDeployModal)}
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
        <div className="glass-table-shell overflow-x-auto" data-testid="workloads-table">
          <table className="w-full min-w-0 border-collapse">
            <thead>
              <tr className="glass-divider-b">
                <th scope="col" className="w-10 py-3 px-2">
                  <input
                    type="checkbox"
                    aria-label="Select all"
                    checked={filteredWorkloads.length > 0 && filteredWorkloads.every((w) => selectedNames.has(w.name))}
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelectedNames(new Set(filteredWorkloads.map((w) => w.name)));
                      } else {
                        setSelectedNames(new Set());
                      }
                    }}
                  />
                </th>
                <th scope="col" className="text-left text-xs uppercase tracking-wider text-slate-400 py-3 px-3">Name</th>
                <th scope="col" className="hidden text-left text-xs uppercase tracking-wider text-slate-400 py-3 px-3 md:table-cell">Runtime</th>
                <th scope="col" className="hidden text-left text-xs uppercase tracking-wider text-slate-400 py-3 px-3 lg:table-cell">Image</th>
                <th scope="col" className="text-left text-xs uppercase tracking-wider text-slate-400 py-3 px-3">Status</th>
                <th scope="col" className="text-left text-xs uppercase tracking-wider text-slate-400 py-3 px-3">Actions</th>
              </tr>
            </thead>
            <tbody>
              {displayWorkloads.map((w) => {
                const shortName = clusterResourceName(w.name);
                const locationLabel = [w.cluster, w.namespace].filter(Boolean).join('/');
                const metaLabel = [w.kind, locationLabel || null].filter(Boolean).join(' · ');
                const discovered = !isAetherManaged(w);
                const showLogs = !discovered || hasClusterLogs(w.kind);
                const showShell = discovered
                  ? canMutate && isShellableClusterKind(w.kind)
                  : canMutate;
                const showShellDenied = discovered
                  ? !canMutate && isShellableClusterKind(w.kind)
                  : !canMutate;
                const pinned = pinnedSet.has(w.name);

                return (
                <tr key={w.name} className="glass-table-row glass-inset-hover transition-colors">
                  <td className="py-3 px-2 align-top">
                    {!discovered && (
                      <input
                        type="checkbox"
                        checked={selectedNames.has(w.name)}
                        onChange={() => toggleSelect(w.name)}
                        aria-label={`Select ${w.name}`}
                      />
                    )}
                  </td>
                  <td className="py-3 px-3 font-medium text-slate-200 align-top min-w-0">
                    <div className="flex items-start gap-1.5 min-w-0">
                      <button
                        type="button"
                        onClick={() => togglePin(w.name)}
                        className={`mt-0.5 shrink-0 transition-colors ${pinned ? 'text-aether' : 'text-slate-600 hover:text-slate-300'}`}
                        title={pinned ? 'Unpin workload' : 'Pin workload'}
                      >
                        <Star size={13} className={pinned ? 'fill-current' : ''} />
                      </button>
                      <button
                        type="button"
                        onClick={() => openWorkloadDetail(w, 'overview')}
                        className="hover:text-aether transition-colors text-left w-full min-w-0 block"
                        title={w.name}
                      >
                        <span className="block truncate">{shortName}</span>
                        {metaLabel ? (
                          <span className="mt-0.5 block truncate text-[11px] font-normal text-slate-500" title={metaLabel}>
                            {metaLabel}
                          </span>
                        ) : null}
                      </button>
                    </div>
                  </td>
                  <td className="hidden py-3 px-3 align-top md:table-cell"><RuntimeBadge runtime={w.runtime} /></td>
                  <td className="hidden py-3 px-3 align-top lg:table-cell min-w-0">
                    <code className="text-xs glass-inset-surface px-2 py-1 rounded text-slate-300 block truncate" title={w.image}>
                      {w.image}
                    </code>
                  </td>
                  <td className="py-3 px-3 align-top whitespace-nowrap">
                    <Badge text={w.status} variant={getStatusVariant(w.status)} />
                  </td>
                  <td className="py-3 px-3 align-top">
                    <div className="flex flex-nowrap items-center gap-0.5">
                      {discovered ? (
                        <>
                          <button type="button" onClick={() => openWorkloadDetail(w, 'overview')} className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-white/5 rounded transition-colors" title="Info"><Info size={14} /></button>
                          {showLogs ? (
                            <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="p-1.5 text-slate-400 hover:text-blue-400 hover:bg-blue-500/10 rounded transition-colors" title="Logs"><FileText size={14} /></button>
                          ) : null}
                          {showShell ? (
                            <button type="button" onClick={() => openWorkloadDetail(w, 'overview', true)} className="p-1.5 text-slate-400 hover:text-emerald-400 hover:bg-emerald-500/10 rounded transition-colors" title="Exec into pod"><Terminal size={14} /></button>
                          ) : showShellDenied ? (
                            <button type="button" disabled className="p-1.5 text-slate-600 cursor-not-allowed rounded" title="Exec requires Operator or Admin role"><Terminal size={14} /></button>
                          ) : null}
                        </>
                      ) : (
                        <>
                          <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="p-1.5 text-slate-400 hover:text-blue-400 hover:bg-blue-500/10 rounded transition-colors" title="Logs"><FileText size={14} /></button>
                          {canMutate ? (
                            <>
                              <button type="button" onClick={() => handleAction(w.name, 'start')} disabled={actionLoading === `${w.name}-start`} className="p-1.5 text-slate-400 hover:text-emerald-400 hover:bg-emerald-500/10 rounded transition-colors" title="Start"><Play size={14} /></button>
                              <button type="button" onClick={() => setConfirmAction({ type: 'stop', name: w.name })} disabled={actionLoading === `${w.name}-stop`} className="p-1.5 text-slate-400 hover:text-amber-400 hover:bg-amber-500/10 rounded transition-colors" title="Stop"><Square size={14} /></button>
                              <button type="button" onClick={() => setConfirmAction({ type: 'delete', name: w.name })} disabled={actionLoading === `${w.name}-delete`} className="p-1.5 text-slate-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors" title="Delete"><Trash2 size={14} /></button>
                            </>
                          ) : null}
                        </>
                      )}
                    </div>
                  </td>
                </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
        </>
      ) : null}

      <Modal
        isOpen={bulkConfirmDelete}
        onClose={() => setBulkConfirmDelete(false)}
        title="Confirm bulk delete"
      >
        <p className="text-sm text-slate-300 mb-6">
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
        <p className="text-sm text-slate-300 mb-6">
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
        <p className="text-sm text-slate-400 mb-4">Select target runtime to load migration advice:</p>
        <div className="grid grid-cols-2 gap-3 mb-6">
          {runtimes.map((rt) => (
            <button
              key={rt}
              type="button"
              onClick={() => migrateModal && void loadMigrationAdvice(migrateModal, rt)}
              disabled={adviceLoading || actionLoading !== null}
              className={`px-4 py-3 border rounded-lg text-sm font-medium transition-colors capitalize ${
                migrateTarget === rt
                  ? 'border-aether/50 bg-aether/10 text-aether'
                  : 'glass-inset-surface glass-inset-hover glass-divider text-slate-200'
              }`}
            >
              {adviceLoading && migrateTarget === rt ? 'Loading advice…' : rt}
            </button>
          ))}
        </div>

        {migrateAdvice && migrateTarget && (
          <div className="glass-panel-card p-4 space-y-3">
            <h4 className="text-sm font-semibold text-slate-100">
              Advice: {migrateAdvice.source_runtime} → {migrateAdvice.target_runtime}
            </h4>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-slate-500">Strategy</span>
                <p className="text-slate-200">{migrateAdvice.recommended_strategy}</p>
              </div>
              <div>
                <span className="text-slate-500">Risk</span>
                <p className="text-slate-200">{migrateAdvice.risk_level}</p>
              </div>
              <div>
                <span className="text-slate-500">Est. downtime</span>
                <p className="text-slate-200">{migrateAdvice.estimated_downtime_secs}s</p>
              </div>
              {migrateAdvice.timing && (
                <div>
                  <span className="text-slate-500">Timing</span>
                  <p className="text-slate-200">{migrateAdvice.timing.recommendation}</p>
                </div>
              )}
            </div>
            <div>
              <span className="text-xs uppercase tracking-wider text-slate-500 mb-2 block">Migration strategy</span>
              <div className="flex flex-wrap gap-2">
                {(['immediate', 'blue-green', 'rolling', 'confidential-blue-green'] as const).map((s) => (
                  <label key={s} className="inline-flex items-center gap-2 text-sm text-slate-300 cursor-pointer">
                    <input
                      type="radio"
                      name="migrate-strategy"
                      value={s}
                      checked={migrateStrategy === s}
                      onChange={() => setMigrateStrategy(s)}
                      className="accent-aether"
                    />
                    {s}
                  </label>
                ))}
              </div>
              {migrateStrategy === 'confidential-blue-green' && (
                <p className="text-xs text-amber-400/90 mt-2">
                  Encrypted migration: deploy target, re-attest launch digest, then cutover. Requires TEE-capable nodes.
                </p>
              )}
              {confidentialMigrationPlan && (
                <div className="mt-3 p-3 rounded-lg glass-inset-surface border glass-divider text-xs space-y-2">
                  <p className="text-slate-400 font-mono break-all">
                    Channel: {confidentialMigrationPlan.encrypted_migration_uri}
                  </p>
                  {confidentialMigrationPlan.blockers.length > 0 ? (
                    <ul className="text-red-300/90">
                      {confidentialMigrationPlan.blockers.map((b) => (
                        <li key={b}>{b}</li>
                      ))}
                    </ul>
                  ) : (
                    <p className="text-emerald-400/90">Ready for confidential cutover</p>
                  )}
                </div>
              )}
            </div>
            {migrateAdvice.reasons.length > 0 && (
              <ul className="text-xs text-slate-400 space-y-1">
                {migrateAdvice.reasons.map((r, i) => (
                  <li key={i}>+ {r}</li>
                ))}
              </ul>
            )}
            {migrateAdvice.warnings.length > 0 && (
              <ul className="text-xs text-amber-400/90 space-y-1">
                {migrateAdvice.warnings.map((w, i) => (
                  <li key={i}>! {w}</li>
                ))}
              </ul>
            )}
            <button
              type="button"
              onClick={() => migrateModal && migrateTarget && handleMigrate(migrateModal, migrateTarget, migrateStrategy)}
              disabled={actionLoading !== null}
              className="w-full py-3 bg-aether hover:bg-aether-light disabled:opacity-50 rounded-xl font-medium text-white"
            >
              {actionLoading ? 'Starting migration…' : `Start migration to ${migrateTarget}`}
            </button>
            {migrateResult ? (
              <p
                data-testid="migrate-result"
                className={`text-sm rounded-lg border px-3 py-2 ${
                  migrateResult.toLowerCase().includes('failed')
                    ? 'border-red-500/30 bg-red-500/5 text-red-300'
                    : 'border-emerald-500/30 bg-emerald-500/5 text-emerald-300'
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

      <Modal isOpen={deployModal} onClose={closeDeployModal} title="Deploy New Workload" size="yaml">
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
            header={
              <div className="shrink-0 rounded-xl border border-purple-500/20 bg-purple-500/5 p-4 space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <h3 className="text-sm font-medium text-purple-300">Confidential assist</h3>
                  <button
                    type="button"
                    onClick={() => setDeployYaml(mergeConfidentialIntoYaml(deployYaml, deployConfidential))}
                    className="rounded-lg border border-purple-500/30 bg-purple-500/10 px-3 py-1.5 text-xs font-medium text-purple-200 hover:bg-purple-500/20"
                  >
                    Apply to YAML
                  </button>
                </div>
                <ConfidentialFormFields
                  runtime="kubernetes"
                  state={deployConfidential}
                  onChange={(field, value) =>
                    setDeployConfidential((prev) => ({ ...prev, [field]: value }))
                  }
                />
              </div>
            }
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
  );
}
