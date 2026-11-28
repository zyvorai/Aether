// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router';
import { ArrowRight, CornerDownRight, History, Layers, Search, Star, Wrench, Zap, type LucideIcon } from 'lucide-react';
import type { AppView, WorkloadResponse } from '../types/api';
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
import { getSectionLabel } from '../utils/navSections';
import { groupByCategory, parseQuery, rankCommands, type PaletteScope, type WorkloadTier } from '../utils/paletteRank';
import { workloadCommands } from '../utils/paletteWorkloads';
import { copyToClipboard } from '../utils/clipboard';
import { usePreferences } from '../hooks/usePreferences';
import { useTheme } from '../contexts/ThemeContext';

type CommandCategory = 'pinned' | 'recent' | 'recent-action' | 'navigation' | 'setup' | 'workload' | 'workload-action' | 'action';

const MAX_RESULTS = 60;

const CATEGORY_LABELS: Record<CommandCategory, string> = {
  pinned: 'Pinned',
  recent: 'Recent pages',
  'recent-action': 'Recent actions',
  navigation: 'Pages',
  setup: 'Setup required',
  workload: 'Workloads',
  'workload-action': 'Workload actions',
  action: 'Actions',
};

const CATEGORY_ICONS: Record<CommandCategory, LucideIcon> = {
  pinned: Star,
  recent: History,
  'recent-action': History,
  navigation: ArrowRight,
  setup: Wrench,
  workload: Layers,
  'workload-action': CornerDownRight,
  action: Zap,
};

const SCOPE_CATEGORIES: Record<Exclude<PaletteScope, 'all'>, CommandCategory[]> = {
  pages: ['pinned', 'recent', 'navigation', 'setup'],
  workloads: ['workload', 'workload-action'],
  actions: ['recent-action', 'action'],
};

/** Offered before anything is typed. */
const START_ACTIONS = ['action-deploy', 'action-validate', 'action-theme', 'action-density'];

const VIEW_META = new Map(DASHBOARD_VIEWS.map((v) => [v.view, v]));

interface CommandAction {
  id: string;
  label: string;
  category: CommandCategory;
  searchText: string;
  view?: AppView;
  workload?: { name: string; tier: WorkloadTier };
  workloadTab?: string;
  status?: string;
  run?: () => void | Promise<void>;
}

import type { HelpTab } from './HelpDialog';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  onNavigate: (view: AppView) => void;
  workloads: WorkloadResponse[];
  onSelectWorkload?: (name: string) => void;
  onRefresh?: () => void;
  onLogout?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
}

const NAV_ITEMS: CommandAction[] = [...DASHBOARD_VIEWS]
  .sort((a, b) => {
    const aHub = a.sidebar === false ? 1 : 0;
    const bHub = b.sidebar === false ? 1 : 0;
    if (aHub !== bHub) return aHub - bHub;
    return a.label.localeCompare(b.label);
  })
  .map((v) => ({
  id: `nav-${v.view}`,
  label: v.view === 'overview' ? 'Command Center' : `Go to ${v.label}`,
  category: 'navigation' as const,
  searchText: `${v.label} ${v.paletteLabel ?? ''} ${v.subtitle} ${v.view} ${v.group}`,
  view: v.view,
}));

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
  const { pinnedViews, density, sidebarCollapsed, setPreferences } = usePreferences();
  const { theme, setTheme, toggleDarkLight } = useTheme();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentViews, setRecentViews] = useState<AppView[]>([]);
  const [recentActionIds, setRecentActionIds] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const runLifecycle = useCallback(
    async (name: string, action: 'start' | 'stop') => {
      if (!canMutate) {
        paletteToast(`Your role cannot ${action} workloads`, 'error');
        return;
      }
      const res = await apiPost(`/workloads/${name}/${action}`);
      paletteToast(
        res.success
          ? `${action === 'start' ? 'Started' : 'Stopped'} "${name}"`
          : `${action === 'start' ? 'Start' : 'Stop'} failed: ${res.error ?? 'unknown error'}`,
        res.success ? 'success' : 'error',
      );
      onRefresh?.();
    },
    [canMutate, onRefresh],
  );

  const allCommands = useMemo((): CommandAction[] => {
    const workloadItems: CommandAction[] = workloads
      .flatMap((w) => workloadCommands(w, { gitopsConfigured: gitopsConfigured === true }))
      .map(({ to, action, ...command }) => ({
        ...command,
        run: action
          ? () => runLifecycle(command.workload.name, action)
          : () => {
              if (!to) return;
              navigate(pathWithQuery(viewToPath(to.view), to.params));
              if (to.select) onSelectWorkload?.(command.workload.name);
            },
      }));

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
        id: 'action-zyra',
        label: 'Open Zyra',
        category: 'action',
        searchText: 'zyra chat assistant natural language',
        view: 'zyra',
      },
      {
        id: 'action-zyra-health',
        label: 'Ask Zyra about fleet health',
        category: 'action',
        searchText: 'copilot health unhealthy workloads fleet fleet health ask',
        run: () =>
          navigate(
            pathWithQuery(viewToPath('zyra'), {
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
        searchText: 'gpu workloads accelerator kubevirt high latency',
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
        id: 'action-ai-os-security-zyra',
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

    const interfaceItems: CommandAction[] = [
      {
        id: 'action-theme',
        label: 'Switch between light and dark theme',
        category: 'action',
        searchText: 'theme appearance light dark mode toggle colour color',
        run: toggleDarkLight,
      },
      ...(theme === 'system'
        ? []
        : [{
            id: 'action-theme-system',
            label: 'Follow the system theme',
            category: 'action' as const,
            searchText: 'theme appearance system automatic os',
            run: () => setTheme('system'),
          }]),
      {
        id: 'action-density',
        label: density === 'compact' ? 'Use comfortable density' : 'Use compact density',
        category: 'action',
        searchText: 'density compact comfortable spacing rows tables appearance',
        run: () => setPreferences({ density: density === 'compact' ? 'comfortable' : 'compact' }),
      },
      {
        id: 'action-sidebar',
        label: sidebarCollapsed ? 'Expand the sidebar' : 'Collapse the sidebar',
        category: 'action',
        searchText: 'sidebar navigation rail collapse expand hide show',
        run: () => setPreferences({ sidebarCollapsed: !sidebarCollapsed }),
      },
      {
        id: 'action-copy-link',
        label: 'Copy link to this view',
        category: 'action',
        searchText: 'copy share link url page filters',
        run: async () => {
          const ok = await copyToClipboard(window.location.href);
          paletteToast(ok ? 'Link copied' : 'Could not copy the link', ok ? 'success' : 'error');
        },
      },
    ];

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

    return [...visibleNav, ...setupCommands, ...workloadItems, ...actionItems, ...interfaceItems];
  }, [workloads, runLifecycle, navigate, onSelectWorkload, onRefresh, onLogout, onOpenHelp, onNavigate, canMutate, capabilities, gitopsConfigured, theme, setTheme, toggleDarkLight, density, sidebarCollapsed, setPreferences]);

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

  const pinnedCommands = useMemo(
    (): CommandAction[] =>
      pinnedViews.flatMap((view) => {
        const meta = DASHBOARD_VIEWS.find((v) => v.view === view);
        return meta
          ? [{ id: `pinned-${view}`, label: meta.label, category: 'pinned' as const, searchText: `pinned ${meta.label}`, view }]
          : [];
      }),
    [pinnedViews],
  );

  const filtered = useMemo(() => {
    const { scope, text: q } = parseQuery(query);
    const inScope = (cmd: CommandAction) => scope === 'all' || SCOPE_CATEGORIES[scope].includes(cmd.category);
    const recentIds = new Set(recentCommands.map((c) => c.id));
    const base = allCommands.filter((cmd) => !recentIds.has(cmd.id));

    let results: CommandAction[];
    if (q) {
      // Search each destination once: recent and pinned copies are for browsing.
      // The page id is searchable too ("deps", "rbac", "sbom").
      const ranked = rankCommands(
        q,
        allCommands.filter(inScope).map((cmd) => (cmd.view ? { ...cmd, searchText: `${cmd.searchText} ${cmd.view}` } : cmd)),
      );
      results = groupByCategory(ranked.slice(0, MAX_RESULTS));
    } else if (scope !== 'all') {
      results = base.filter(inScope).slice(0, MAX_RESULTS);
    } else {
      const shortcuts = base.filter((cmd) => START_ACTIONS.includes(cmd.id));
      const pages = base.filter((cmd) => cmd.category === 'navigation');
      results = [...pinnedCommands, ...recentCommands, ...shortcuts, ...pages].slice(0, MAX_RESULTS);
    }

    const ql = q.toLowerCase();
    const scaleMatch = ql.match(/^scale\s+(.+)$/);
    const whyMatch = ql.match(/^why\s+(.+?)\s+failing\??$/);

    const contextual: CommandAction[] = [];
    const names = workloads.map((w) => w.name);
    const resolveName = (needle: string) =>
      names.find((n) => n.toLowerCase() === needle) ?? names.find((n) => n.toLowerCase().includes(needle));

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
              pathWithQuery(viewToPath('zyra'), {
                workload: match,
                q: `Why is ${match} failing?`,
              }),
            ),
        });
      }
    }

    if (contextual.length > 0) {
      const seen = new Set(contextual.map((c) => c.id));
      results = [...contextual, ...results.filter((c) => !seen.has(c.id))].slice(0, MAX_RESULTS);
    }

    return results;
  }, [allCommands, query, recentCommands, pinnedCommands, workloads, navigate]);

  useEffect(() => {
    if (!open) return;
    const opener = document.activeElement as HTMLElement | null;
    setQuery('');
    setSelectedIndex(0);
    setRecentViews(getRecentViews());
    setRecentActionIds(getRecentActions().map((a) => a.id));
    inputRef.current?.focus();
    return () => {
      // Hand focus back unless the chosen command moved it somewhere else.
      if (opener?.isConnected && (document.activeElement === document.body || document.activeElement === inputRef.current)) {
        opener.focus();
      }
    };
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
      // Pages are remembered by the recent-pages list; this one is for actions.
      if (!cmd.view) pushRecentAction({ id: cmd.id, label: cmd.label, searchText: cmd.searchText || cmd.label });
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
    const step = (delta: number) => {
      if (filtered.length === 0) return;
      setSelectedIndex((i) => (i + delta + filtered.length) % filtered.length);
    };
    if (e.key === 'ArrowDown' || (e.ctrlKey && e.key === 'n')) {
      e.preventDefault();
      step(1);
    } else if (e.key === 'ArrowUp' || (e.ctrlKey && e.key === 'p')) {
      e.preventDefault();
      step(-1);
    } else if (e.key === 'Home' && !query) {
      e.preventDefault();
      setSelectedIndex(0);
    } else if (e.key === 'End' && !query) {
      e.preventDefault();
      setSelectedIndex(Math.max(0, filtered.length - 1));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      e.preventDefault();
      void executeCommand(filtered[selectedIndex]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Tab') {
      // The list is driven from the search field, so focus stays there.
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    }
  };

  if (!open) return null;

  let lastCategory: CommandCategory | '' = '';
  const selected = filtered[selectedIndex];

  return (
    <div
      data-testid="command-palette"
      className="palette-root"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
    >
      <div className="fixed inset-0 backdrop-blur-xs" style={{ background: 'var(--scrim)' }} aria-hidden />
      <div className="glass-drawer palette-panel" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-2.5 border-b border-rule px-4 py-3.5">
          <Search className="h-4 w-4 shrink-0 text-subtle" aria-hidden />
          <input
            ref={inputRef}
            type="text"
            data-testid="command-palette-input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search pages, workloads, and actions…"
            aria-label="Search pages, workloads, and actions"
            aria-controls="command-palette-results"
            className="min-w-0 flex-1 bg-transparent text-[15px] text-foreground outline-hidden placeholder:text-subtle"
            autoComplete="off"
            spellCheck={false}
          />
          <button type="button" onClick={onClose} className="palette-key" aria-label="Close command palette">
            Esc
          </button>
        </div>

        <p className="sr-only" role="status" aria-live="polite">
          {selected ? `${selected.label}, ${selectedIndex + 1} of ${filtered.length}` : 'No results'}
        </p>

        <div ref={listRef} id="command-palette-results" className="max-h-[min(28rem,55dvh)] overflow-y-auto px-2 py-1.5">
          {filtered.length === 0 ? (
            <div data-testid="command-palette-empty" className="px-4 py-8 text-center text-sm text-subtle">
              No results found
              <span className="mt-1 block text-xs">Try fewer words, or start with / for pages, @ for workloads, &gt; for actions.</span>
            </div>
          ) : (
            filtered.map((cmd, i) => {
              const showCategory = cmd.category !== lastCategory;
              lastCategory = cmd.category;
              const meta = cmd.view ? VIEW_META.get(cmd.view) : undefined;
              const Icon = meta?.icon ?? CATEGORY_ICONS[cmd.category];
              const hint = meta ? getSectionLabel(meta.view) : (cmd.status ?? cmd.workloadTab);
              return (
                <div key={cmd.id}>
                  {showCategory ? <div className="palette-heading">{CATEGORY_LABELS[cmd.category]}</div> : null}
                  <button
                    type="button"
                    tabIndex={-1}
                    data-testid={`command-palette-item-${cmd.id}`}
                    onClick={() => void executeCommand(cmd)}
                    onMouseMove={() => setSelectedIndex(i)}
                    data-selected={i === selectedIndex}
                    aria-current={i === selectedIndex ? 'true' : undefined}
                    className="palette-item"
                  >
                    <Icon className="h-4 w-4 shrink-0 text-subtle" strokeWidth={1.7} aria-hidden />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate">{cmd.label}</span>
                      {meta && i === selectedIndex ? <span className="palette-sub">{meta.subtitle}</span> : null}
                    </span>
                    {hint ? <span className="shrink-0 text-xs text-subtle">{hint}</span> : null}
                  </button>
                </div>
              );
            })
          )}
        </div>

        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-rule px-4 py-2.5 text-xs text-subtle">
          <span><kbd>↑</kbd> <kbd>↓</kbd> move</span>
          <span><kbd>Enter</kbd> select</span>
          <span><kbd>/</kbd> pages</span>
          <span><kbd>@</kbd> workloads</span>
          <span><kbd>&gt;</kbd> actions</span>
          <span className="ml-auto">{isMac ? '⌘K' : 'Ctrl+K'}</span>
        </div>
      </div>
    </div>
  );
}
