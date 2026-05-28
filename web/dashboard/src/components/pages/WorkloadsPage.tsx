// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router';
import { Play, Square, ArrowRightLeft, Trash2, FileText, Cpu, Search, ClipboardCheck, RefreshCw, Inbox, Hammer, Plus, Rocket, FileCode2, Layers } from 'lucide-react';
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
import { formatTimestamp } from '../../utils/formatters';
import Badge, { RuntimeBadge } from '../Badge';
import StatCard from '../StatCard';
import Modal from '../Modal';
import YamlInput from '../YamlInput';
import ValidateResultPanel from '../ValidateResultPanel';
import DeploySuccessPanel from '../DeploySuccessPanel';
import EmptyState from '../EmptyState';
import type { WorkloadResponse, ValidateResponse, BuildResponse, MigrationAdvice, PolicyResult, ConfidentialMigrationPlan } from '../../types/api';
import PageToolbar from '../PageToolbar';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import WorkloadDetail, { type DetailTab } from '../WorkloadDetail';

interface WorkloadsPageProps {
  initialSelectedName?: string | null;
  onClearInitialSelection?: () => void;
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

export default function WorkloadsPage({ initialSelectedName, onClearInitialSelection }: WorkloadsPageProps) {
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

  const runtimes = ['podman', 'docker', 'kubernetes', 'kubevirt', 'metal3'];

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

  useEffect(() => { load(); }, []);

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
    const tab = (['overview', 'logs', 'manifest', 'drift', 'scoring', 'events', 'trust'] as const).includes(
      tabParam as DetailTab,
    )
      ? (tabParam as DetailTab)
      : 'overview';
    setDetailInitialTab(tab);
    setSelectedWorkload(match);
  }, [workloadParam, tabParam, workloads]);

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

  async function handleBuild(name: string) {
    setActionLoading(`${name}-build`);
    const res = await apiPost<BuildResponse>(`/workloads/${name}/build`);
    setActionLoading(null);
    if (res.success && res.data) {
      toast(`Build "${name}" succeeded: ${res.data.full_name}`, 'success');
    } else {
      toast(`Build "${name}" failed: ${res.error ?? 'unknown error'}`, 'error');
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

  function openWorkloadDetail(workload: WorkloadResponse, tab: DetailTab) {
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
    return matchesSearch && matchesSource && matchesKind && matchesCluster && matchesNamespace && matchesStatus;
    }),
  );

  const aetherManagedCount = countAetherManaged(workloads);
  const clusterDiscoveredCount = workloads.length - aetherManagedCount;
  const runningCount = workloads.filter((w) => {
    const s = w.status.toLowerCase();
    return s === 'running' || s.includes('deployed');
  }).length;
  const stoppedCount = workloads.filter((w) => ['stopped', 'exited'].includes(w.status.toLowerCase())).length;

  const filterSelectClass = 'rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-zinc-100';

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Search name, image, cluster, namespace..."
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
          </>
        }
        actions={
          canMutate ? (
          <>
            <button
              type="button"
              data-testid="workloads-deploy-button"
              onClick={() => openDeployModal(setDeployYaml, setDeployConfidential, setDeployModal)}
              className="inline-flex items-center gap-2 rounded-xl bg-aether px-3 py-2 text-sm font-medium text-white transition hover:bg-aether-light"
            >
              <Plus size={16} />
              Deploy
            </button>
            <button
              type="button"
              data-testid="workloads-validate-button"
              onClick={() => setValidateModal(true)}
              className="inline-flex items-center gap-2 rounded-xl bg-blue-600 px-3 py-2 text-sm font-medium text-white transition hover:bg-blue-500"
            >
              <ClipboardCheck size={16} />
              Validate
            </button>
          </>
          ) : null
        }
      />

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
        </WorkloadContextBanner>
      ) : null}

      {sourceFilterVal !== 'all' ? (
        <div className="mb-4 rounded-xl border border-slate-700/80 bg-slate-950/50 px-4 py-3 text-sm text-slate-400">
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

      <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4 mb-6">
        <StatCard title="Total" value={workloads.length} color="orange" />
        <button type="button" data-testid="workloads-running-stat" onClick={() => setStatusFilter('running')} className="text-left">
          <StatCard title="Running" value={runningCount} color="green" />
        </button>
        <button type="button" onClick={() => setStatusFilter('stopped')} className="text-left">
          <StatCard title="Stopped" value={stoppedCount} color="red" />
        </button>
        <button type="button" data-testid="workloads-aether-stat" onClick={() => setSourceFilter('aether')} className="text-left">
          <StatCard title="Aether" value={aetherManagedCount} color="purple" />
        </button>
        <button type="button" data-testid="workloads-discovered-stat" onClick={() => setSourceFilter('cluster')} className="text-left">
          <StatCard title="Discovered" value={clusterDiscoveredCount} color="blue" />
        </button>
        <StatCard
          title="Namespaces"
          value={namespaces.filter((namespace) => namespace !== 'all').length}
          color="green"
        />
      </div>

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
                  className="inline-flex items-center gap-2 rounded-xl bg-aether px-4 py-2 text-sm font-medium text-white hover:bg-aether/90 transition-colors"
                >
                  <Plus className="h-4 w-4" />
                  Deploy YAML
                </button>
                <button
                  type="button"
                  onClick={() => navigate(viewToPath('editor'))}
                  className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
                >
                  <FileCode2 className="h-4 w-4" />
                  Visual Editor
                </button>
                <button
                  type="button"
                  onClick={() => navigate(viewToPath('compose'))}
                  className="inline-flex items-center gap-2 rounded-xl border border-slate-700 px-4 py-2 text-sm text-slate-300 hover:border-aether/40 hover:text-aether transition-colors"
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
      ) : (
        <div className="dash-card-flush" data-testid="workloads-table">
          <div className="overflow-x-auto min-w-0">
            <table className="w-full table-fixed border-collapse">
              <thead>
                <tr className="border-b border-zinc-800">
                  <th className="w-10 py-3 px-2">
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
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[18%] min-w-0">Name</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[12%] min-w-0">Runtime</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[36%] min-w-0">Image</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[10%] min-w-0">Status</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[14%] min-w-0">Created</th>
                  <th className="text-left text-xs uppercase tracking-wider text-zinc-400 py-3 px-3 sm:px-4 w-[10%] min-w-0">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredWorkloads.map((w) => (
                  <tr key={w.name} className="border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors">
                    <td className="py-3 px-2 align-top">
                      {isAetherManaged(w) && (
                        <input
                          type="checkbox"
                          checked={selectedNames.has(w.name)}
                          onChange={() => toggleSelect(w.name)}
                          aria-label={`Select ${w.name}`}
                        />
                      )}
                    </td>
                    <td className="py-3 px-3 sm:px-4 font-medium text-zinc-200 align-top min-w-0">
                      <button
                        type="button"
                        onClick={() => openWorkloadDetail(w, 'overview')}
                        className="hover:text-aether transition-colors text-left w-full min-w-0 truncate block"
                        title={w.name}
                      >
                        {w.name}
                      </button>
                    </td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0"><RuntimeBadge runtime={w.runtime} /></td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <div className="space-y-1 min-w-0">
                        <code
                          className="text-xs bg-zinc-950 px-2 py-1 rounded text-zinc-300 block w-full min-w-0 truncate"
                          title={w.image}
                        >
                          {w.image}
                        </code>
                        {(w.cluster || w.namespace || w.kind) && (
                          <div
                            className="text-[11px] text-zinc-500 truncate"
                            title={[w.kind, w.cluster, w.namespace].filter(Boolean).join(' · ')}
                          >
                            {[w.kind, w.cluster, w.namespace].filter(Boolean).join(' · ')}
                          </div>
                        )}
                      </div>
                    </td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <Badge text={w.status} variant={getStatusVariant(w.status)} />
                    </td>
                    <td className="py-3 px-3 sm:px-4 text-sm text-zinc-400 align-top min-w-0 whitespace-nowrap">{formatTimestamp(w.created_at)}</td>
                    <td className="py-3 px-3 sm:px-4 align-top min-w-0">
                      <div className="flex flex-wrap gap-1">
                        {isAetherManaged(w) ? (
                          <>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'logs')} className="p-1.5 text-zinc-400 hover:text-blue-400 hover:bg-blue-500/10 rounded transition-colors" title="Logs"><FileText size={14} /></button>
                            {canMutate ? (
                              <>
                            <button type="button" onClick={() => handleAction(w.name, 'start')} disabled={actionLoading === `${w.name}-start`} className="p-1.5 text-zinc-400 hover:text-emerald-400 hover:bg-emerald-500/10 rounded transition-colors" title="Start"><Play size={14} /></button>
                            <button type="button" onClick={() => setConfirmAction({ type: 'stop', name: w.name })} disabled={actionLoading === `${w.name}-stop`} className="p-1.5 text-zinc-400 hover:text-amber-400 hover:bg-amber-500/10 rounded transition-colors" title="Stop"><Square size={14} /></button>
                            <button type="button" onClick={() => handleBuild(w.name)} disabled={actionLoading === `${w.name}-build`} className="p-1.5 text-zinc-400 hover:text-teal-400 hover:bg-teal-500/10 rounded transition-colors" title="Build"><Hammer size={14} /></button>
                            <button type="button" onClick={() => setUpdateModal(w.name)} className="p-1.5 text-zinc-400 hover:text-sky-400 hover:bg-sky-500/10 rounded transition-colors" title="Update spec"><FileText size={14} /></button>
                            <button type="button" onClick={() => setMigrateModal(w.name)} className="p-1.5 text-zinc-400 hover:text-purple-400 hover:bg-purple-500/10 rounded transition-colors" title="Migrate"><ArrowRightLeft size={14} /></button>
                              </>
                            ) : null}
                            <button type="button" onClick={() => openWorkloadDetail(w, 'scoring')} className="p-1.5 text-zinc-400 hover:text-cyan-400 hover:bg-cyan-500/10 rounded transition-colors" title="Profile"><Cpu size={14} /></button>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'scoring')} className="p-1.5 text-zinc-400 hover:text-indigo-400 hover:bg-indigo-500/10 rounded transition-colors" title="Analyze"><Search size={14} /></button>
                            <button type="button" onClick={() => openWorkloadDetail(w, 'drift')} className="p-1.5 text-zinc-400 hover:text-orange-400 hover:bg-orange-500/10 rounded transition-colors" title="Drift"><RefreshCw size={14} /></button>
                            {canMutate ? (
                            <button type="button" onClick={() => setConfirmAction({ type: 'delete', name: w.name })} disabled={actionLoading === `${w.name}-delete`} className="p-1.5 text-zinc-400 hover:text-red-400 hover:bg-red-500/10 rounded transition-colors" title="Delete"><Trash2 size={14} /></button>
                            ) : null}
                          </>
                        ) : (
                          <span className="text-xs text-zinc-500 px-2 py-1">
                            K8s discovered
                          </span>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {selectedWorkload && (
        <WorkloadDetail
          key={`${selectedWorkload.name}-${detailInitialTab}`}
          workload={selectedWorkload}
          initialTab={detailInitialTab}
          canMutate={canMutate}
          onClose={closeWorkloadDetail}
          onMigrate={isAetherManaged(selectedWorkload) ? (name) => setMigrateModal(name) : undefined}
          onAction={() => load()}
        />
      )}

      <Modal
        isOpen={bulkConfirmDelete}
        onClose={() => setBulkConfirmDelete(false)}
        title="Confirm bulk delete"
      >
        <p className="text-sm text-zinc-300 mb-6">
          Delete {selectedNames.size} selected workload(s)? This cannot be undone.
        </p>
        <div className="flex justify-end gap-3">
          <button type="button" onClick={() => setBulkConfirmDelete(false)} className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-sm font-medium transition-colors">
            Cancel
          </button>
          <button type="button" onClick={() => void bulkDelete()} className="px-4 py-2 rounded-lg text-sm font-medium text-white bg-red-600 hover:bg-red-500 transition-colors">
            Delete all
          </button>
        </div>
      </Modal>

      <Modal
        isOpen={confirmAction !== null}
        onClose={() => setConfirmAction(null)}
        title={confirmAction?.type === 'delete' ? 'Confirm Delete' : 'Confirm Stop'}
      >
        <p className="text-sm text-zinc-300 mb-6">
          {confirmAction?.type === 'delete'
            ? `Are you sure you want to delete "${confirmAction.name}"?`
            : `Stop workload "${confirmAction?.name}"?`}
        </p>
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setConfirmAction(null)}
            className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 rounded-lg text-sm font-medium transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleConfirmedAction}
            className={`px-4 py-2 rounded-lg text-sm font-medium text-white transition-colors ${
              confirmAction?.type === 'delete'
                ? 'bg-red-600 hover:bg-red-500'
                : 'bg-amber-600 hover:bg-amber-500'
            }`}
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
        <p className="text-sm text-zinc-400 mb-4">Select target runtime to load migration advice:</p>
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
                  : 'bg-zinc-800 hover:bg-zinc-700 border-zinc-700 text-zinc-200'
              }`}
            >
              {adviceLoading && migrateTarget === rt ? 'Loading advice…' : rt}
            </button>
          ))}
        </div>

        {migrateAdvice && migrateTarget && (
          <div className="rounded-xl border border-zinc-700 bg-zinc-950/60 p-4 space-y-3">
            <h4 className="text-sm font-semibold text-zinc-100">
              Advice: {migrateAdvice.source_runtime} → {migrateAdvice.target_runtime}
            </h4>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <span className="text-zinc-500">Strategy</span>
                <p className="text-zinc-200">{migrateAdvice.recommended_strategy}</p>
              </div>
              <div>
                <span className="text-zinc-500">Risk</span>
                <p className="text-zinc-200">{migrateAdvice.risk_level}</p>
              </div>
              <div>
                <span className="text-zinc-500">Est. downtime</span>
                <p className="text-zinc-200">{migrateAdvice.estimated_downtime_secs}s</p>
              </div>
              {migrateAdvice.timing && (
                <div>
                  <span className="text-zinc-500">Timing</span>
                  <p className="text-zinc-200">{migrateAdvice.timing.recommendation}</p>
                </div>
              )}
            </div>
            <div>
              <span className="text-xs uppercase tracking-wider text-zinc-500 mb-2 block">Migration strategy</span>
              <div className="flex flex-wrap gap-2">
                {(['immediate', 'blue-green', 'rolling', 'confidential-blue-green'] as const).map((s) => (
                  <label key={s} className="inline-flex items-center gap-2 text-sm text-zinc-300 cursor-pointer">
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
                <div className="mt-3 p-3 rounded-lg bg-zinc-950 border border-zinc-800 text-xs space-y-2">
                  <p className="text-zinc-400 font-mono break-all">
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
