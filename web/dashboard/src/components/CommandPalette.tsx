// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router';
import type { AppView } from '../types/api';
import { DASHBOARD_VIEWS } from '../utils/dashboardNav';
import { viewToPath } from '../utils/dashboardRoutes';
import { pathWithQuery } from '../utils/urlState';
import { apiPost } from '../utils/api';
import { getRecentViews } from '../utils/recentViews';
import { getRecentActions, pushRecentAction } from '../utils/recentActions';
import { useAuth } from '../contexts/AuthContext';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';
import { partitionNavViews } from '../utils/navCapabilities';
import { inferFixRecommendations } from '../utils/k8sUx';

type CommandCategory = 'recent' | 'recent-action' | 'navigation' | 'setup' | 'workload' | 'workload-action' | 'action';

interface CommandAction {
  id: string;
  label: string;
  category: CommandCategory;
  searchText: string;
  view?: AppView;
  workloadName?: string;
  workloadTab?: string;
  run?: () => void | Promise<void>;
}

import type { HelpTab } from './HelpDialog';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: AppView) => void;
  workloads: string[];
  onSelectWorkload?: (name: string) => void;
  onRefresh?: () => void;
  onLogout?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
}

const NAV_ITEMS: CommandAction[] = DASHBOARD_VIEWS.map((v) => ({
  id: `nav-${v.view}`,
  label: v.view === 'overview' ? 'Command Center' : `Go to ${v.label}`,
  category: 'navigation' as const,
  searchText: `${v.label} ${v.subtitle} ${v.view}`,
  view: v.view,
}));

function fuzzyScore(query: string, text: string): number {
  const q = query.toLowerCase().trim();
  const t = text.toLowerCase();
  if (!q) return 1;
  if (t.includes(q)) return 100 + (t.startsWith(q) ? 20 : 0) + (t === q ? 30 : 0);
  let qi = 0;
  let score = 0;
  for (let i = 0; i < t.length && qi < q.length; i++) {
    if (t[i] === q[qi]) {
      score += 10 - Math.min(i, 5);
      qi++;
    }
  }
  return qi === q.length ? score : 0;
}

function paletteToast(message: string, type: 'success' | 'error') {
  window.dispatchEvent(new CustomEvent('aether-toast', { detail: { message, type } }));
}

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPod|iPad/i.test(navigator.platform);

export default function CommandPalette({
  open,
  onClose,
  onNavigate,
  workloads,
  onSelectWorkload,
  onRefresh,
  onLogout,
  onOpenHelp,
}: CommandPaletteProps) {
  const navigate = useNavigate();
  const { canMutate } = useAuth();
  const { capabilities, gitopsConfigured } = useServerCapabilities();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentViews, setRecentViews] = useState<AppView[]>([]);
  const [recentActionIds, setRecentActionIds] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const allCommands = useMemo((): CommandAction[] => {
    const workloadItems: CommandAction[] = workloads.flatMap((name) => [
      {
        id: `application-${name}`,
        label: `Open application: ${name}`,
        category: 'workload' as const,
        searchText: `application app kubernetes ${name}`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('applications'), { workload: name }));
        },
      },
      {
        id: `workload-${name}`,
        label: `Open workload: ${name}`,
        category: 'workload' as const,
        searchText: `workload ${name}`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-logs`,
        label: `View logs: ${name}`,
        category: 'workload-action' as const,
        searchText: `logs ${name} workload`,
        workloadName: name,
        workloadTab: 'logs',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'logs' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-drift`,
        label: `Check drift: ${name}`,
        category: 'workload-action' as const,
        searchText: `drift ${name} workload reconcile`,
        workloadName: name,
        workloadTab: 'drift',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'drift' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-events`,
        label: `View events: ${name}`,
        category: 'workload-action' as const,
        searchText: `events ${name} workload`,
        workloadName: name,
        workloadTab: 'events',
        run: () => {
          navigate(pathWithQuery(viewToPath('events'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-copilot`,
        label: `Ask copilot about: ${name}`,
        category: 'workload-action' as const,
        searchText: `copilot ask health ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(
            pathWithQuery(viewToPath('copilot'), {
              workload: name,
              q: `Why is ${name} unhealthy?`,
            }),
          );
        },
      },
      {
        id: `workload-${name}-trust`,
        label: `Trust tab: ${name}`,
        category: 'workload-action' as const,
        searchText: `trust attestation guestkit tee confidential ${name}`,
        workloadName: name,
        workloadTab: 'trust',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'trust' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-confidential`,
        label: `Confidential fleet: ${name}`,
        category: 'workload-action' as const,
        searchText: `confidential fleet tee ${name}`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('confidential'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-alerts`,
        label: `Alert rules: ${name}`,
        category: 'workload-action' as const,
        searchText: `alerts webhooks ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('alerts'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-audit`,
        label: `Audit trail: ${name}`,
        category: 'workload-action' as const,
        searchText: `audit trail history ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('audit'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-health`,
        label: `Health monitor: ${name}`,
        category: 'workload-action' as const,
        searchText: `health monitor liveness ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('health'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-gitops`,
        label: `GitOps sync: ${name}`,
        category: 'workload-action' as const,
        searchText: `gitops sync reconcile ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('gitops'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-metrics`,
        label: `Metrics: ${name}`,
        category: 'workload-action' as const,
        searchText: `metrics grafana prometheus ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('metrics'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-deps`,
        label: `Dependencies: ${name}`,
        category: 'workload-action' as const,
        searchText: `dependencies graph startup ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('deps'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-affinity`,
        label: `Runtime affinity: ${name}`,
        category: 'workload-action' as const,
        searchText: `affinity runtime class ${name} workload`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('affinity'), { workload: name }));
        },
      },
      {
        id: `workload-${name}-sla`,
        label: `SLA compliance: ${name}`,
        category: 'workload-action' as const,
        searchText: `sla uptime compliance ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('sla'), { workload: name })),
      },
      {
        id: `workload-${name}-envs`,
        label: `Environments: ${name}`,
        category: 'workload-action' as const,
        searchText: `environments promote ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('envs'), { workload: name })),
      },
      {
        id: `workload-${name}-templates`,
        label: `Templates: ${name}`,
        category: 'workload-action' as const,
        searchText: `templates scaffold ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('templates'), { workload: name })),
      },
      {
        id: `workload-${name}-plugins`,
        label: `Plugins: ${name}`,
        category: 'workload-action' as const,
        searchText: `plugins runtime ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('plugins'), { workload: name })),
      },
      {
        id: `workload-${name}-cost`,
        label: `Cost estimate: ${name}`,
        category: 'workload-action' as const,
        searchText: `cost chargeback ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('cost'), { workload: name })),
      },
      {
        id: `workload-${name}-scheduler`,
        label: `Scheduler: ${name}`,
        category: 'workload-action' as const,
        searchText: `scheduler placement ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('scheduler'), { workload: name })),
      },
      {
        id: `workload-${name}-policy`,
        label: `Policy check: ${name}`,
        category: 'workload-action' as const,
        searchText: `policy opa validate ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('policy'), { workload: name })),
      },
      {
        id: `workload-${name}-intelligence`,
        label: `Intelligence: ${name}`,
        category: 'workload-action' as const,
        searchText: `intelligence predictions threats ${name}`,
        workloadName: name,
        run: () =>
          navigate(pathWithQuery(viewToPath('intelligence'), { workload: name, tab: 'predictions' })),
      },
      {
        id: `workload-${name}-editor`,
        label: `Visual editor: ${name}`,
        category: 'workload-action' as const,
        searchText: `editor visual yaml ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('editor'), { workload: name })),
      },
      {
        id: `workload-${name}-platform`,
        label: `Platform context: ${name}`,
        category: 'workload-action' as const,
        searchText: `platform ha integrations ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('platform'), { workload: name })),
      },
      {
        id: `workload-${name}-rbac`,
        label: `RBAC keys: ${name}`,
        category: 'workload-action' as const,
        searchText: `rbac api keys roles ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('rbac'), { workload: name })),
      },
      {
        id: `workload-${name}-openapi`,
        label: `OpenAPI routes: ${name}`,
        category: 'workload-action' as const,
        searchText: `openapi api routes swagger ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('openapi'), { workload: name })),
      },
      {
        id: `workload-${name}-clusters`,
        label: `Cluster browser: ${name}`,
        category: 'workload-action' as const,
        searchText: `clusters kubernetes browse ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('clusters'), { workload: name })),
      },
      {
        id: `workload-${name}-compose`,
        label: `Compose stack: ${name}`,
        category: 'workload-action' as const,
        searchText: `compose docker stack import ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('compose'), { workload: name })),
      },
      {
        id: `workload-${name}-fleet-scoped`,
        label: `Fleet view: ${name}`,
        category: 'workload-action' as const,
        searchText: `fleet multi cluster ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('fleet'), { workload: name })),
      },
      {
        id: `workload-${name}-backups`,
        label: `Backups: ${name}`,
        category: 'workload-action' as const,
        searchText: `backup restore snapshot ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('backups'), { workload: name })),
      },
      {
        id: `workload-${name}-secrets`,
        label: `Secrets: ${name}`,
        category: 'workload-action' as const,
        searchText: `secrets vault keys ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('secrets'), { workload: name })),
      },
      {
        id: `workload-${name}-ai`,
        label: `AI engine: ${name}`,
        category: 'workload-action' as const,
        searchText: `ai scoring intent ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('ai'), { workload: name, tab: 'analyze' })),
      },
      {
        id: `workload-${name}-drift-page`,
        label: `Drift page: ${name}`,
        category: 'workload-action' as const,
        searchText: `drift reconcile page ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('drift'), { workload: name })),
      },
      {
        id: `workload-${name}-audit-trail`,
        label: `Audit trail: ${name}`,
        category: 'workload-action' as const,
        searchText: `audit compliance trail ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('audit'), { workload: name })),
      },
      {
        id: `workload-${name}-policy-scoped`,
        label: `Policy check: ${name}`,
        category: 'workload-action' as const,
        searchText: `policy opa validate ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('policy'), { workload: name })),
      },
      {
        id: `workload-${name}-backups-restore`,
        label: `Backups: ${name}`,
        category: 'workload-action' as const,
        searchText: `backup restore snapshot ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('backups'), { workload: name })),
      },
      {
        id: `workload-${name}-compose-deps`,
        label: `Compose deps: ${name}`,
        category: 'workload-action' as const,
        searchText: `compose dependencies stack ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('deps'), { workload: name })),
      },
      {
        id: `workload-${name}-fleet-hub`,
        label: `Fleet hub: ${name}`,
        category: 'workload-action' as const,
        searchText: `fleet multi cluster hub ${name}`,
        workloadName: name,
        run: () => navigate(pathWithQuery(viewToPath('fleet'), { workload: name })),
      },
      {
        id: `workload-${name}-scoring`,
        label: `Open scoring: ${name}`,
        category: 'workload-action' as const,
        searchText: `scoring ai intent ${name}`,
        workloadName: name,
        workloadTab: 'scoring',
        run: () => {
          navigate(pathWithQuery(viewToPath('workloads'), { workload: name, tab: 'scoring' }));
          onSelectWorkload?.(name);
        },
      },
      {
        id: `workload-${name}-start`,
        label: `Start workload: ${name}`,
        category: 'workload-action' as const,
        searchText: `start run ${name}`,
        workloadName: name,
        run: async () => {
          if (!canMutate) return;
          const res = await apiPost(`/workloads/${name}/start`);
          paletteToast(
            res.success ? `Started "${name}"` : `Start failed: ${res.error ?? 'unknown error'}`,
            res.success ? 'success' : 'error',
          );
          onRefresh?.();
        },
      },
      {
        id: `workload-${name}-scale`,
        label: `Scale application: ${name}`,
        category: 'workload-action' as const,
        searchText: `scale replicas application ${name}`,
        workloadName: name,
        run: () => {
          navigate(pathWithQuery(viewToPath('applications'), { workload: name, tab: 'overview' }));
        },
      },
      {
        id: `workload-${name}-why-failing`,
        label: `Why is ${name} failing?`,
        category: 'workload-action' as const,
        searchText: `why failing troubleshoot diagnose ${name}`,
        workloadName: name,
        run: () => {
          navigate(
            pathWithQuery(viewToPath('copilot'), {
              workload: name,
              q: `Why is ${name} failing?`,
            }),
          );
        },
      },
      {
        id: `workload-${name}-stop`,
        label: `Stop workload: ${name}`,
        category: 'workload-action' as const,
        searchText: `stop halt ${name}`,
        workloadName: name,
        run: async () => {
          if (!canMutate) return;
          const res = await apiPost(`/workloads/${name}/stop`);
          paletteToast(
            res.success ? `Stopped "${name}"` : `Stop failed: ${res.error ?? 'unknown error'}`,
            res.success ? 'success' : 'error',
          );
          onRefresh?.();
        },
      },
    ]);

    const actionItems: CommandAction[] = [];

    if (canMutate) {
      actionItems.unshift({
        id: 'action-deploy',
        label: 'Deploy workload',
        category: 'action',
        searchText: 'deploy yaml create workload',
        run: () => navigate(pathWithQuery(viewToPath('workloads'), { deploy: '1' })),
      });
      actionItems.unshift({
        id: 'action-validate',
        label: 'Validate workload YAML',
        category: 'action',
        searchText: 'validate yaml lint check',
        run: () => navigate(pathWithQuery(viewToPath('workloads'), { validate: '1' })),
      });
    }

    actionItems.push(
      {
        id: 'action-refresh',
        label: 'Refresh dashboard',
        category: 'action',
        searchText: 'refresh reload sync',
        run: () => onRefresh?.(),
      },
      {
        id: 'action-editor',
        label: 'Open visual editor',
        category: 'action',
        searchText: 'editor visual form designer',
        view: 'editor',
      },
      {
        id: 'action-compose',
        label: 'Import Docker Compose',
        category: 'action',
        searchText: 'compose docker import',
        view: 'compose',
      },
      {
        id: 'action-fleet',
        label: 'Open fleet overview',
        category: 'action',
        searchText: 'fleet multi-cluster inventory',
        view: 'fleet',
      },
      {
        id: 'action-drift',
        label: 'Open drift detection',
        category: 'action',
        searchText: 'drift reconcile configuration',
        view: 'drift',
      },
      {
        id: 'action-drift-scan',
        label: 'Open drift bulk scan',
        category: 'action',
        searchText: 'drift scan bulk reconcile all workloads',
        run: () => navigate(viewToPath('drift')),
      },
      {
        id: 'action-alerts',
        label: 'Open alerts & webhooks',
        category: 'action',
        searchText: 'alerts webhooks notifications',
        view: 'alerts',
      },
      {
        id: 'action-openapi',
        label: 'Open API explorer',
        category: 'action',
        searchText: 'openapi api routes swagger',
        view: 'openapi',
      },
      {
        id: 'action-scheduler',
        label: 'Open placement scheduler',
        category: 'action',
        searchText: 'scheduler placement affinity',
        view: 'scheduler',
      },
      {
        id: 'action-secrets',
        label: 'Open secrets vault',
        category: 'action',
        searchText: 'secrets vault keys rotation',
        view: 'secrets',
      },
      {
        id: 'action-backups',
        label: 'Open backups',
        category: 'action',
        searchText: 'backup restore snapshot',
        view: 'backups',
      },
      {
        id: 'action-ai',
        label: 'Open AI engine',
        category: 'action',
        searchText: 'ai recommend scoring intent runtime',
        view: 'ai',
      },
      {
        id: 'action-intelligence',
        label: 'Open intelligence reports',
        category: 'action',
        searchText: 'intelligence predictions threats cost placement',
        view: 'intelligence',
      },
      {
        id: 'action-copilot',
        label: 'Open ops copilot',
        category: 'action',
        searchText: 'copilot chat assistant natural language',
        view: 'copilot',
      },
      {
        id: 'action-copilot-health',
        label: 'Ask copilot about fleet health',
        category: 'action',
        searchText: 'copilot health unhealthy workloads fleet fleet health ask',
        run: () =>
          navigate(
            pathWithQuery(viewToPath('copilot'), {
              q: 'Why is my workload unhealthy?',
            }),
          ),
      },
      {
        id: 'action-ai-os-fabric',
        label: 'Open runtime fabric graph',
        category: 'action',
        searchText: 'fabric topology graph runtime infrastructure live',
        view: 'fabric',
      },
      {
        id: 'action-ai-os-command-center',
        label: 'Open command center',
        category: 'action',
        searchText: 'command center briefing overview fleet health savings',
        view: 'overview',
      },
      {
        id: 'action-ai-os-migrations',
        label: 'Open AI migration planner',
        category: 'action',
        searchText: 'migration migrate move workload failed migrations plan',
        view: 'migrations',
      },
      {
        id: 'action-ai-os-gpu',
        label: 'Find GPU workloads',
        category: 'action',
        searchText: 'gpu workloads accelerator kubevirt metal3 high latency',
        run: () => navigate(pathWithQuery(viewToPath('workloads'), { q: 'gpu' })),
      },
      {
        id: 'action-ai-os-cost-waste',
        label: 'Find workloads wasting resources',
        category: 'action',
        searchText: 'waste cost optimization oversized savings finops',
        run: () => navigate(pathWithQuery(viewToPath('cost'), { tab: 'optimize' })),
      },
      {
        id: 'action-ai-os-sla',
        label: 'Show SLA violations',
        category: 'action',
        searchText: 'sla violations compliance breach uptime',
        view: 'sla',
      },
      {
        id: 'action-ai-os-capacity',
        label: 'Open capacity forecast',
        category: 'action',
        searchText: 'capacity forecast saturation gpu utilization scheduler',
        view: 'observability',
      },
      {
        id: 'action-ai-os-digital-twin',
        label: 'Run digital twin simulation',
        category: 'action',
        searchText: 'digital twin what-if simulate capacity cost risk fabric',
        view: 'fabric',
      },
      {
        id: 'action-ai-os-security-copilot',
        label: 'Generate security policies',
        category: 'action',
        searchText: 'security copilot policy least privilege network hardening',
        view: 'security',
      },
      {
        id: 'action-ai-os-cost-intelligence',
        label: 'Open cost intelligence',
        category: 'action',
        searchText: 'cost intelligence finops savings optimize waste',
        view: 'cost',
      },
      {
        id: 'action-ai-os-autonomy',
        label: 'Open autonomous mode settings',
        category: 'action',
        searchText: 'autonomous mode self healing auto restart drift agents',
        view: 'settings',
      },
      {
        id: 'action-ai-os-self-healing',
        label: 'Preview self-healing actions',
        category: 'action',
        searchText: 'self healing healer orchestrator remediation autonomous sre',
        view: 'observability',
      },
      {
        id: 'action-ai-os-knowledge-graph',
        label: 'Open infrastructure knowledge graph',
        category: 'action',
        searchText: 'knowledge graph dependencies threats drift impact analysis labs',
        view: 'labs',
      },
      {
        id: 'action-ai-os-graph-search',
        label: 'Search knowledge graph nodes',
        category: 'action',
        searchText: 'graph search nodes workload threat dependency cmdk',
        run: () => navigate(pathWithQuery(viewToPath('labs'), { tab: 'graph-search' })),
      },
      {
        id: 'action-ai-os-intent-pipeline',
        label: 'Run intent to infrastructure pipeline',
        category: 'action',
        searchText: 'intent infrastructure pipeline outcome deploy spec placement',
        run: () => navigate(pathWithQuery(viewToPath('ai'), { tab: 'pipeline' })),
      },
      {
        id: 'action-ai-os-sre-runbook',
        label: 'Generate SRE runbook',
        category: 'action',
        searchText: 'sre runbook autonomous operations incident response',
        view: 'observability',
      },
      {
        id: 'action-ai-os-multicloud',
        label: 'Open multi-cloud posture',
        category: 'action',
        searchText: 'multicloud federation cluster placement anomalies fleet',
        view: 'fleet',
      },
      {
        id: 'action-ai-os-autonomous-placement',
        label: 'Review autonomous placement',
        category: 'action',
        searchText: 'autonomous placement runtime evolution migrate auto eligible',
        view: 'migrations',
      },
      {
        id: 'action-affinity',
        label: 'Open runtime affinity',
        category: 'action',
        searchText: 'affinity runtime class matrix',
        view: 'affinity',
      },
      {
        id: 'action-cost',
        label: 'Open cost estimation',
        category: 'action',
        searchText: 'cost estimate pricing chargeback',
        view: 'cost',
      },
      {
        id: 'action-confidential',
        label: 'Open confidential computing',
        category: 'action',
        searchText: 'confidential tee attestation kata',
        view: 'confidential',
      },
      {
        id: 'action-trust-attestation',
        label: 'Open trust & attestation',
        category: 'action',
        searchText: 'trust attestation guestkit tee confidential fleet',
        view: 'confidential',
      },
      {
        id: 'action-health',
        label: 'Open health monitor',
        category: 'action',
        searchText: 'health monitor rolling update liveness',
        view: 'health',
      },
      {
        id: 'action-policy',
        label: 'Open policy check',
        category: 'action',
        searchText: 'policy opa admission validate',
        view: 'policy',
      },
      {
        id: 'action-templates',
        label: 'Open workload templates',
        category: 'action',
        searchText: 'templates library scaffold',
        view: 'templates',
      },
      {
        id: 'action-templates-configure',
        label: 'Configure workload template',
        category: 'action',
        searchText: 'templates configure scaffold parameters',
        run: () => navigate(pathWithQuery(viewToPath('templates'), { configure: '1' })),
      },
      {
        id: 'action-gitops',
        label: 'Open GitOps sync',
        category: 'action',
        searchText: 'gitops sync repository reconcile',
        view: 'gitops',
      },
      {
        id: 'action-envs',
        label: 'Open environments',
        category: 'action',
        searchText: 'environments tiers promote parity',
        view: 'envs',
      },
      {
        id: 'action-rbac',
        label: 'Open access control',
        category: 'action',
        searchText: 'rbac api keys roles admin',
        view: 'rbac',
      },
      {
        id: 'action-plugins',
        label: 'Open plugins',
        category: 'action',
        searchText: 'plugins runtime extensions discover',
        view: 'plugins',
      },
      {
        id: 'action-audit',
        label: 'Open audit trail',
        category: 'action',
        searchText: 'audit log trail events history',
        view: 'audit',
      },
      {
        id: 'action-events',
        label: 'Open events feed',
        category: 'action',
        searchText: 'events notifications feed alerts',
        view: 'events',
      },
      {
        id: 'action-sla-events',
        label: 'Open SLA events feed',
        category: 'action',
        searchText: 'sla events compliance breaches',
        run: () => navigate(pathWithQuery(viewToPath('events'), { category: 'sla' })),
      },
      {
        id: 'action-intent-debugger',
        label: 'Open AI intent debugger',
        category: 'action',
        searchText: 'intent debugger violations scoring ai',
        run: () => navigate(pathWithQuery(viewToPath('ai'), { tab: 'analyze' })),
      },
      {
        id: 'action-sla',
        label: 'Open SLA compliance',
        category: 'action',
        searchText: 'sla uptime compliance targets',
        view: 'sla',
      },
      {
        id: 'action-metrics',
        label: 'Open metrics & Grafana',
        category: 'action',
        searchText: 'metrics grafana prometheus chargeback',
        view: 'metrics',
      },
      {
        id: 'action-platform',
        label: 'Open platform & HA',
        category: 'action',
        searchText: 'platform ha opa cilium integrations',
        view: 'platform',
      },
      {
        id: 'action-clusters',
        label: 'Open cluster browser',
        category: 'action',
        searchText: 'clusters kubernetes browse namespaces network',
        view: 'clusters',
      },
      {
        id: 'action-clusters-pods',
        label: 'Browse cluster pods',
        category: 'action',
        searchText: 'clusters pods browse kubernetes',
        run: () => navigate(pathWithQuery(viewToPath('clusters'), { kind: 'Pod' })),
      },
      {
        id: 'action-compose-validate',
        label: 'Validate compose stack',
        category: 'action',
        searchText: 'compose validate import stack yaml',
        view: 'compose',
      },
      {
        id: 'action-audit-failures',
        label: 'Open audit failures',
        category: 'action',
        searchText: 'audit failures errors trail',
        run: () => navigate(pathWithQuery(viewToPath('audit'), { result: 'failure' })),
      },
      {
        id: 'action-clusters-network',
        label: 'Browse cluster network policies',
        category: 'action',
        searchText: 'clusters network policies cilium browse',
        run: () => navigate(pathWithQuery(viewToPath('clusters'), { tab: 'network' })),
      },
      {
        id: 'action-events-health',
        label: 'Open health events',
        category: 'action',
        searchText: 'events health monitor alerts',
        run: () => navigate(pathWithQuery(viewToPath('events'), { category: 'health' })),
      },
      {
        id: 'action-deps',
        label: 'Open dependencies graph',
        category: 'action',
        searchText: 'dependencies graph startup order',
        view: 'deps',
      },
    );

    if (onLogout) {
      actionItems.push({
        id: 'action-logout',
        label: 'Sign out',
        category: 'action',
        searchText: 'logout sign out exit',
        run: () => onLogout(),
      });
    }

    if (onOpenHelp) {
      actionItems.push(
        {
          id: 'action-help-shortcuts',
          label: 'Help: keyboard shortcuts',
          category: 'action',
          searchText: 'help shortcuts keyboard ?',
          run: () => onOpenHelp('shortcuts'),
        },
        {
          id: 'action-help-about',
          label: 'Help: about Aether',
          category: 'action',
          searchText: 'help about aether zyvor copyright documentation',
          run: () => onOpenHelp('about'),
        },
      );
    }

    const platform = capabilities?.platform ?? null;
    const navMeta = DASHBOARD_VIEWS.map((v) => ({
      view: v.view,
      label: v.paletteLabel ?? v.label,
    }));
    const { ready, setup } = partitionNavViews(navMeta, platform, { gitopsConfigured });

    const visibleNav = NAV_ITEMS.filter(
      (item) => !item.view || ready.some((r) => r.view === item.view),
    );

    const setupCommands: CommandAction[] = setup.map((item) => ({
      id: `setup-${item.view}`,
      label: `Setup: ${item.label}`,
      category: 'setup' as const,
      searchText: `setup configure platform ${item.label} ${item.visibility.setupHint ?? ''}`,
      run: () => {
        onNavigate('platform');
      },
    }));

    return [...visibleNav, ...setupCommands, ...workloadItems, ...actionItems];
  }, [workloads, navigate, onSelectWorkload, onRefresh, onLogout, onOpenHelp, onNavigate, canMutate, capabilities, gitopsConfigured]);

  const recentCommands = useMemo((): CommandAction[] => {
    const items: CommandAction[] = [];
    for (const id of recentActionIds) {
      const cmd = allCommands.find((c) => c.id === id);
      if (cmd) {
        items.push({ ...cmd, category: 'recent-action' });
      }
    }
    for (const view of recentViews) {
      const meta = DASHBOARD_VIEWS.find((v) => v.view === view);
      if (!meta) continue;
      items.push({
        id: `recent-${view}`,
        label: meta.label,
        category: 'recent',
        searchText: `recent ${meta.label} ${meta.subtitle} ${view}`,
        view,
      });
    }
    return items;
  }, [recentViews, recentActionIds, allCommands]);

  const filtered = useMemo(() => {
    const q = query.trim();
    const recentIds = new Set(recentCommands.map((c) => c.id));
    const base = allCommands.filter((cmd) => !recentIds.has(cmd.id));
    const pool = q ? [...recentCommands, ...base] : [...recentCommands, ...base];
    const scored = pool
      .map((cmd) => ({ cmd, score: fuzzyScore(q, cmd.searchText || cmd.label) }))
      .filter(({ score }) => score > 0)
      .sort((a, b) => b.score - a.score);
    let results = (q ? scored.map(({ cmd }) => cmd) : pool).slice(0, 40);

    const ql = q.toLowerCase();
    const scaleMatch = ql.match(/^scale\s+(.+)$/);
    const whyMatch = ql.match(/^why\s+(.+?)\s+failing\??$/);

    const contextual: CommandAction[] = [];
    const resolveName = (needle: string) =>
      workloads.find((w) => w.toLowerCase() === needle) ??
      workloads.find((w) => w.toLowerCase().includes(needle));

    if (scaleMatch) {
      const match = resolveName(scaleMatch[1].trim().toLowerCase());
      if (match) {
        contextual.push({
          id: `context-scale-${match}`,
          label: `Scale ${match}`,
          category: 'action',
          searchText: q,
          run: () => navigate(pathWithQuery(viewToPath('applications'), { workload: match, tab: 'overview' })),
        });
      }
    }

    if (whyMatch) {
      const match = resolveName(whyMatch[1].trim().toLowerCase());
      if (match) {
        const stub = { name: match, runtime: 'kubernetes', image: '', status: 'error', created_at: '' };
        const fix = inferFixRecommendations(stub);
        contextual.push({
          id: `context-why-${match}`,
          label: fix ? `Why ${match} failing: ${fix.title}` : `Why is ${match} failing?`,
          category: 'action',
          searchText: q,
          run: () =>
            navigate(
              pathWithQuery(viewToPath('copilot'), {
                workload: match,
                q: `Why is ${match} failing?`,
              }),
            ),
        });
      }
    }

    if (contextual.length > 0) {
      const seen = new Set(contextual.map((c) => c.id));
      results = [...contextual, ...results.filter((c) => !seen.has(c.id))].slice(0, 40);
    }

    return results;
  }, [allCommands, query, recentCommands, workloads, navigate]);

  useEffect(() => {
    if (open) {
      setQuery('');
      setSelectedIndex(0);
      setRecentViews(getRecentViews());
      setRecentActionIds(getRecentActions().map((a) => a.id));
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector('[data-selected="true"]');
    selected?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  const executeCommand = useCallback(
    async (cmd: CommandAction) => {
      pushRecentAction({ id: cmd.id, label: cmd.label, searchText: cmd.searchText || cmd.label });
      if (cmd.view) {
        onNavigate(cmd.view);
      } else if (cmd.run) {
        await cmd.run();
      }
      onClose();
    },
    [onNavigate, onClose],
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      void executeCommand(filtered[selectedIndex]);
    } else if (e.key === 'Escape') {
      onClose();
    }
  };

  if (!open) return null;

  const categoryLabels: Record<CommandCategory, string> = {
    recent: 'Recent pages',
    'recent-action': 'Recent actions',
    navigation: 'Navigation',
    setup: 'Setup required',
    workload: 'Workloads',
    'workload-action': 'Workload actions',
    action: 'Actions',
  };

  let lastCategory: CommandCategory | '' = '';

  return (
    <div
      data-testid="command-palette"
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] px-4"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <div className="fixed inset-0 bg-[#0a0d12]/80 backdrop-blur-md" />
      <div
        className="overview-section-shell relative w-full max-w-xl overflow-hidden shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className={`flex items-center px-4 py-4 border-b ${'border-slate-800'}`}>
          <span className="text-slate-500 mr-2 text-sm font-mono">{'>'}</span>
          <input
            ref={inputRef}
            type="text"
            data-testid="command-palette-input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search pages, workloads, and actions…"
            className={`flex-1 bg-transparent text-sm outline-none ${'text-white placeholder-slate-500'}`}
            autoComplete="off"
          />
          <kbd className={`text-xs px-1.5 py-0.5 rounded border ${'text-slate-500 bg-slate-800/90 border-slate-700'}`}>ESC</kbd>
        </div>

        <div ref={listRef} className="max-h-[min(24rem,50vh)] overflow-y-auto py-1">
          {filtered.length === 0 ? (
            <div data-testid="command-palette-empty" className="px-4 py-8 text-center text-slate-500 text-sm">
              No results found
            </div>
          ) : (
            filtered.map((cmd, i) => {
              const showCategory = cmd.category !== lastCategory;
              lastCategory = cmd.category;
              return (
                <div key={cmd.id}>
                  {showCategory ? (
                    <div className="px-4 pt-2 pb-1 text-xs font-medium text-slate-500 uppercase tracking-wider">
                      {categoryLabels[cmd.category]}
                    </div>
                  ) : null}
                  <button
                    type="button"
                    data-testid={`command-palette-item-${cmd.id}`}
                    onClick={() => void executeCommand(cmd)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    data-selected={i === selectedIndex}
                    className={`w-full px-4 py-2 flex items-center gap-3 text-sm text-left transition-colors ${
                      i === selectedIndex
                        ? 'bg-aether/20 text-aether'
                        : 'text-slate-300 hover:bg-slate-800/80'
                    }`}
                  >
                    <span className="flex-1 truncate">{cmd.label}</span>
                    {cmd.view ? <span className="text-xs text-slate-600 shrink-0">Navigate</span> : null}
                    {cmd.workloadTab ? <span className="text-xs text-slate-600 shrink-0">{cmd.workloadTab}</span> : null}
                  </button>
                </div>
              );
            })
          )}
        </div>

        <div className={`px-4 py-3 border-t flex flex-wrap items-center gap-x-4 gap-y-1 text-xs ${'border-slate-800 text-slate-500'}`}>
          <span>{isMac ? '⌘K' : 'Ctrl+K'} open</span>
          <span>↑↓ navigate</span>
          <span>Enter select</span>
          <span>Esc close</span>
        </div>
      </div>
    </div>
  );
}
