// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation, useSearchParams } from 'react-router';
import type { AppView } from './types/api';
import DashboardShell from './components/DashboardShell';
import ErrorBoundary from './components/ErrorBoundary';
import HelpDialog, { type HelpTab } from './components/HelpDialog';
import { useToast } from './components/Toast';
import { useEventStream } from './hooks/useEventStream';
import { useKeyboard } from './hooks/useKeyboard';
import { useSequenceShortcuts } from './hooks/useSequenceShortcut';
import { appShellClass } from './utils/themeSurface';
import { ServerCapabilitiesProvider } from './contexts/ServerCapabilitiesContext';
import { WorkspaceProvider } from './contexts/WorkspaceContext';
import { AuthProvider } from './contexts/AuthContext';
import { pathToView, viewToPath } from './utils/dashboardRoutes';
import { pathWithQuery } from './utils/urlState';
import type { UniversalLinkResolveReport } from './types/api';
import { syncMacOSLiveActivity, subscribeMacOSNavigate } from './utils/macosBridge';
import { HERO_CONFIG } from './utils/dashboardNav';
import { apiFetch, getDevBootstrapApiKey, DEFAULT_DASHBOARD_USERNAME, apiTryCookieSession, getDashboardAuthMode, apiTryAuth, UNAUTHORIZED_EVENT } from './utils/api';
import CommandPalette from './components/CommandPalette';
import { pushRecentView } from './utils/recentViews';
import LoginGate from './components/LoginGate';

// Page components — each is written by the pages agent
import FabricPage from './components/pages/FabricPage';
import MigrationsPage from './components/pages/MigrationsPage';
import ObservabilityPage from './components/pages/ObservabilityPage';
import LabsPage from './components/pages/LabsPage';
import SettingsPage from './components/pages/SettingsPage';
import OverviewPage from './components/pages/OverviewPage';
import ApplicationsPage from './components/pages/ApplicationsPage';
import WorkloadsPage from './components/pages/WorkloadsPage';
import ClustersPage from './components/pages/ClustersPage';
import ComposePage from './components/pages/ComposePage';
import AIPage from './components/pages/AIPage';
import ZyraPage from './components/pages/ZyraPage';
import AiProvidersPage from './components/pages/AiProvidersPage';
import CostPage from './components/pages/CostPage';
import AffinityPage from './components/pages/AffinityPage';
import DriftPage from './components/pages/DriftPage';
import PolicyPage from './components/pages/PolicyPage';
import SchedulerPage from './components/pages/SchedulerPage';
import HealthPage from './components/pages/HealthPage';
import EventsPage from './components/pages/EventsPage';
import AlertsPage from './components/pages/AlertsPage';
import PlatformPage from './components/pages/PlatformPage';
import SLAPage from './components/pages/SLAPage';
import DepsPage from './components/pages/DepsPage';
import EnvsPage from './components/pages/EnvsPage';
import SecretsPage from './components/pages/SecretsPage';
import BackupsPage from './components/pages/BackupsPage';
import StoragePage from './components/pages/StoragePage';
import ForgePage from './components/pages/ForgePage';
import TemplatesPage from './components/pages/TemplatesPage';
import PluginsPage from './components/pages/PluginsPage';
import RbacPage from './components/pages/RbacPage';
import AuditPage from './components/pages/AuditPage';
import MetricsPage from './components/pages/MetricsPage';
import GitOpsPage from './components/pages/GitOpsPage';
import EditorPage from './components/pages/EditorPage';
import IntelligencePage from './components/pages/IntelligencePage';
import FleetPage from './components/pages/FleetPage';
import ActivityMonitorPage from './components/pages/ActivityMonitorPage';
import SecurityCenterPage from './components/pages/SecurityCenterPage';
import HelmCatalogPage from './components/pages/HelmCatalogPage';
import HostedPage from './components/pages/HostedPage';
import OpenApiPage from './components/pages/OpenApiPage';

import ObsRootCausePage from './components/pages/ObsRootCausePage';
import ObsCapacityForecastPage from './components/pages/ObsCapacityForecastPage';
import ObsCapacityScalePage from './components/pages/ObsCapacityScalePage';
import ObsSelfHealingPage from './components/pages/ObsSelfHealingPage';
import ObsAutonomousSrePage from './components/pages/ObsAutonomousSrePage';
import ObsExtensionsPage from './components/pages/ObsExtensionsPage';
import ObsReliabilityPage from './components/pages/ObsReliabilityPage';
import SettingsIdentityPage from './components/pages/SettingsIdentityPage';
import SettingsNavPage from './components/pages/SettingsNavPage';
import SettingsAutonomousPage from './components/pages/SettingsAutonomousPage';
import SettingsMacosPage from './components/pages/SettingsMacosPage';
import LabsLivePage from './components/pages/LabsLivePage';
import LabsGraduationPage from './components/pages/LabsGraduationPage';
import LabsGraphPage from './components/pages/LabsGraphPage';
import LabsPlatformPage from './components/pages/LabsPlatformPage';
import MigPlannerPage from './components/pages/MigPlannerPage';
import MigPlacementPage from './components/pages/MigPlacementPage';
import MigReplicationPage from './components/pages/MigReplicationPage';
import MigWavesPage from './components/pages/MigWavesPage';
import FabricTwinPage from './components/pages/FabricTwinPage';
import FabricTopologyPage from './components/pages/FabricTopologyPage';
import FabricGraphPage from './components/pages/FabricGraphPage';
import FabricUnifiedPage from './components/pages/FabricUnifiedPage';

import AiAdvisorPage from './components/pages/AiAdvisorPage';
import AiDesignerPage from './components/pages/AiDesignerPage';
import AiIntentPage from './components/pages/AiIntentPage';
import AiPipelinePage from './components/pages/AiPipelinePage';
import AiRecommendPage from './components/pages/AiRecommendPage';
import AiOptimizePage from './components/pages/AiOptimizePage';
import AiAnalyzePage from './components/pages/AiAnalyzePage';
import IntelPredictionsPage from './components/pages/IntelPredictionsPage';
import IntelThreatsPage from './components/pages/IntelThreatsPage';
import IntelCostPage from './components/pages/IntelCostPage';
import IntelEvolutionPage from './components/pages/IntelEvolutionPage';
import IntelPlacementPage from './components/pages/IntelPlacementPage';
import FleetOverviewPage from './components/pages/FleetOverviewPage';
import FleetEdgePage from './components/pages/FleetEdgePage';
import FleetPlacementPage from './components/pages/FleetPlacementPage';

import ActivityCpuPage from './components/pages/ActivityCpuPage';
import ActivityMemoryPage from './components/pages/ActivityMemoryPage';
import ActivityRestartsPage from './components/pages/ActivityRestartsPage';
import ActivityErrorsPage from './components/pages/ActivityErrorsPage';
import AffinityRecommendPage from './components/pages/AffinityRecommendPage';
import AffinityMatrixPage from './components/pages/AffinityMatrixPage';
import AffinityStatsPage from './components/pages/AffinityStatsPage';
import ClustersBrowsePage from './components/pages/ClustersBrowsePage';
import ClustersNetworkPage from './components/pages/ClustersNetworkPage';
import PlatformRecommendationsPage from './components/pages/PlatformRecommendationsPage';
import PlatformTrustPage from './components/pages/PlatformTrustPage';
import PlatformEcosystemPage from './components/pages/PlatformEcosystemPage';
import PlatformRuntimePage from './components/pages/PlatformRuntimePage';
import PlatformCiliumPage from './components/pages/PlatformCiliumPage';
import PlatformObservabilityPage from './components/pages/PlatformObservabilityPage';
import MetricsSummaryPage from './components/pages/MetricsSummaryPage';
import MetricsChargebackPage from './components/pages/MetricsChargebackPage';
import MetricsObservabilityPage from './components/pages/MetricsObservabilityPage';
import MetricsPrometheusPage from './components/pages/MetricsPrometheusPage';
import SecurityOverviewPage from './components/pages/SecurityOverviewPage';
import SecurityCopilotPage from './components/pages/SecurityCopilotPage';
import SecurityPlatformPage from './components/pages/SecurityPlatformPage';
import SecuritySbomPage from './components/pages/SecuritySbomPage';
import SecurityRemediationPage from './components/pages/SecurityRemediationPage';
import SecurityThreatsPage from './components/pages/SecurityThreatsPage';
import GitopsCenterPage from './components/pages/GitopsCenterPage';
import GitopsAgentPage from './components/pages/GitopsAgentPage';
import GitopsIntentPage from './components/pages/GitopsIntentPage';
import GitopsSyncPage from './components/pages/GitopsSyncPage';
import CostIntelligencePage from './components/pages/CostIntelligencePage';
import CostFinopsPage from './components/pages/CostFinopsPage';
import CostEstimatePage from './components/pages/CostEstimatePage';
import AlertsChannelsPage from './components/pages/AlertsChannelsPage';
import AlertsRulesPage from './components/pages/AlertsRulesPage';
import AlertsTestPage from './components/pages/AlertsTestPage';
import AlertsQueuePage from './components/pages/AlertsQueuePage';

function AetherDashboard() {
  const navigate = useNavigate();
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const breadcrumbWorkload = searchParams.get('workload')?.trim() || undefined;

  const currentView = useMemo(() => pathToView(location.pathname), [location.pathname]);

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [authBootstrapping, setAuthBootstrapping] = useState(true);
  const [loginNotice, setLoginNotice] = useState<string | undefined>(undefined);
  const [username, setUsername] = useState('');
  const [refreshKey, setRefreshKey] = useState(0);
  const [lastRefreshed, setLastRefreshed] = useState<Date>(new Date());
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [helpTab, setHelpTab] = useState<HelpTab>('shortcuts');
  const [workloadNames, setWorkloadNames] = useState<string[]>([]);
  const [selectedWorkloadFromPalette, setSelectedWorkloadFromPalette] = useState<string | null>(null);
  const [sseBannerVisible, setSseBannerVisible] = useState(false);
  const { toast, ToastContainer } = useToast();
  const sseWasConnectedRef = useRef(false);
  const sseDisconnectedAtRef = useRef<number | null>(null);
  const sseDisconnectToastShownRef = useRef(false);

  useEffect(() => {
    const run = async () => {
      const hadTokenParam = searchParams.has('token');
      const stripTokenFromUrl = () => {
        if (!hadTokenParam) return;
        const params = new URLSearchParams(searchParams);
        params.delete('token');
        const search = params.toString();
        navigate(
          { pathname: location.pathname, search: search ? `?${search}` : '' },
          { replace: true },
        );
      };

      const tokenFromUrl = searchParams.get('token')?.trim();
      if (tokenFromUrl) {
        const result = await apiTryAuth(tokenFromUrl);
        if (result.ok) {
          const u = DEFAULT_DASHBOARD_USERNAME;
          sessionStorage.setItem(
            'aether_auth',
            JSON.stringify({
              authenticated: true,
              username: u,
              token: tokenFromUrl,
              authMode: 'bearer',
            }),
          );
          setIsAuthenticated(true);
          setUsername(u);
          setAuthBootstrapping(false);
          stripTokenFromUrl();
          return;
        }
      }

      const stored = sessionStorage.getItem('aether_auth');
      if (stored) {
        try {
          const parsed = JSON.parse(stored) as {
            authenticated?: unknown;
            username?: unknown;
          };
          if (parsed.authenticated && typeof parsed.username === 'string') {
            setIsAuthenticated(true);
            setUsername(parsed.username);
            setAuthBootstrapping(false);
            stripTokenFromUrl();
            return;
          }
        } catch {
          sessionStorage.removeItem('aether_auth');
        }
      }

      const cookieUser = await apiTryCookieSession();
      if (cookieUser) {
        setIsAuthenticated(true);
        setUsername(cookieUser.username);
        sessionStorage.setItem(
          'aether_auth',
          JSON.stringify({
            authenticated: true,
            username: cookieUser.username,
            authMode: 'cookie',
          }),
        );
        setAuthBootstrapping(false);
        stripTokenFromUrl();
        return;
      }

      const bootstrap = getDevBootstrapApiKey();
      if (bootstrap !== undefined) {
        setIsAuthenticated(true);
        setUsername(DEFAULT_DASHBOARD_USERNAME);
        sessionStorage.setItem(
          'aether_auth',
          JSON.stringify({
            authenticated: true,
            username: DEFAULT_DASHBOARD_USERNAME,
            token: bootstrap,
            authMode: 'dev',
          }),
        );
        setAuthBootstrapping(false);
        stripTokenFromUrl();
        return;
      }

      setIsAuthenticated(false);
      setAuthBootstrapping(false);
      stripTokenFromUrl();
    };
    void run();
  }, [searchParams, location.pathname, navigate]);

  // A 401 from any API call means the session's credential is no longer valid — most
  // commonly because an RBAC key was just created, which permanently ends the open
  // local-dev bypass for every session, including the one that created the key. Drop back
  // to the login screen instead of leaving every page stuck on a generic error with no
  // way to recover.
  useEffect(() => {
    const onUnauthorized = () => {
      if (!isAuthenticated) return;
      sessionStorage.removeItem('aether_auth');
      setLoginNotice('Session expired or no longer authorized — please sign in again.');
      setIsAuthenticated(false);
    };
    window.addEventListener(UNAUTHORIZED_EVENT, onUnauthorized);
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, onUnauthorized);
  }, [isAuthenticated]);

  // Real-time updates via SSE
  const { connected: sseConnected } = useEventStream('', (event) => {
    if (event.type === 'workloadChanged' || event.type === 'healthUpdate') {
      setRefreshKey((k) => k + 1);
      setLastRefreshed(new Date());
    }
    if (event.type === 'migrationProgress') {
      window.dispatchEvent(new CustomEvent('aether-live-activity', { detail: event }));
      const payload = event.payload as { workload?: string; progress?: number; message?: string } | undefined;
      if (payload?.workload) {
        void syncMacOSLiveActivity(
          payload.workload,
          payload.progress ?? 0,
          payload.message ?? 'Migration in progress',
        );
      }
      setRefreshKey((k) => k + 1);
    }
  }, isAuthenticated);

  useEffect(() => {
    const openPalette = () => setCommandPaletteOpen(true);
    window.addEventListener('aether-open-command-palette', openPalette);
    window.addEventListener('aether-open-spotlight', openPalette);
    (window as Window & { __AETHER_OPEN_COMMAND_PALETTE__?: () => void }).__AETHER_OPEN_COMMAND_PALETTE__ =
      openPalette;
    return () => {
      window.removeEventListener('aether-open-command-palette', openPalette);
      window.removeEventListener('aether-open-spotlight', openPalette);
      delete (window as Window & { __AETHER_OPEN_COMMAND_PALETTE__?: () => void }).__AETHER_OPEN_COMMAND_PALETTE__;
    };
  }, []);

  useEffect(() => {
    const onDeepLink = async (e: Event) => {
      const url = (e as CustomEvent<{ url?: string }>).detail?.url;
      if (!url) return;
      const resolved = await apiFetch<UniversalLinkResolveReport>(
        `/intelligence/macos/universal-links/resolve?url=${encodeURIComponent(url)}`,
      );
      if (!resolved?.view) return;
      const params = Object.fromEntries(
        Object.entries(resolved.query).map(([k, v]) => [k, v]),
      ) as Record<string, string | undefined>;
      navigate(pathWithQuery(viewToPath(resolved.view as AppView), params));
    };
    window.addEventListener('aether-deep-link', onDeepLink);
    return () => window.removeEventListener('aether-deep-link', onDeepLink);
  }, [navigate]);

  useEffect(() => {
    return subscribeMacOSNavigate((path) => {
      navigate(path);
      window.scrollTo({ top: 0, behavior: 'smooth' });
    });
  }, [navigate]);

  useEffect(() => {
    if (!isAuthenticated) {
      sseWasConnectedRef.current = false;
      sseDisconnectedAtRef.current = null;
      sseDisconnectToastShownRef.current = false;
      setSseBannerVisible(false);
      return;
    }

    if (sseConnected) {
      if (sseDisconnectedAtRef.current !== null) {
        const offlineMs = Date.now() - sseDisconnectedAtRef.current;
        if (offlineMs >= 10_000) {
          toast('Live updates reconnected', 'info');
        }
        sseDisconnectedAtRef.current = null;
        sseDisconnectToastShownRef.current = false;
      }
      setSseBannerVisible(false);
      sseWasConnectedRef.current = true;
      return;
    }

    if (sseWasConnectedRef.current && sseDisconnectedAtRef.current === null) {
      sseDisconnectedAtRef.current = Date.now();
    }
  }, [sseConnected, isAuthenticated, toast]);

  useEffect(() => {
    if (!isAuthenticated || sseConnected) return;

    const interval = window.setInterval(() => {
      const since = sseDisconnectedAtRef.current;
      if (since === null || sseDisconnectToastShownRef.current) return;
      if (Date.now() - since >= 10_000) {
        toast('Live updates disconnected — falling back to 60s polling', 'info');
        sseDisconnectToastShownRef.current = true;
        setSseBannerVisible(true);
      }
    }, 2000);

    return () => window.clearInterval(interval);
  }, [isAuthenticated, sseConnected, toast]);

  // Auto-refresh every 60 seconds (fallback since SSE provides instant updates)
  useEffect(() => {
    if (!isAuthenticated) return;
    const interval = setInterval(() => {
      setRefreshKey((k) => k + 1);
      setLastRefreshed(new Date());
    }, 60000);
    return () => clearInterval(interval);
  }, [isAuthenticated]);

  // Fetch workload names for command palette
  useEffect(() => {
    if (!isAuthenticated) return;
    apiFetch<Array<{ name: string }>>('/workloads').then((r) => {
      if (r) setWorkloadNames(r.map((w) => w.name));
    });
  }, [isAuthenticated, refreshKey]);

  const handleNavigate = useCallback(
    (view: AppView) => {
      navigate(viewToPath(view));
      window.scrollTo({ top: 0, behavior: 'smooth' });
    },
    [navigate],
  );

  useEffect(() => {
    if (isAuthenticated && currentView) {
      pushRecentView(currentView);
    }
  }, [currentView, isAuthenticated]);

  useEffect(() => {
    const onPageToast = (event: Event) => {
      const detail = (event as CustomEvent<{ message?: string; type?: 'success' | 'error' | 'info' }>).detail;
      if (detail?.message) {
        toast(detail.message, detail.type ?? 'info');
      }
    };
    window.addEventListener('aether-toast', onPageToast);
    return () => window.removeEventListener('aether-toast', onPageToast);
  }, [toast]);

  const openHelp = useCallback((tab: HelpTab = 'shortcuts') => {
    setHelpTab(tab);
    setHelpOpen(true);
  }, []);

  const toggleHelp = useCallback(() => {
    setHelpOpen((open) => {
      if (open) return false;
      setHelpTab('shortcuts');
      return true;
    });
  }, []);

  // Keyboard shortcuts
  useKeyboard({
    onRefresh: () => {
      setRefreshKey((k) => k + 1);
      setLastRefreshed(new Date());
    },
    onCommandPalette: () => setCommandPaletteOpen((o) => !o),
    onOpenHelp: toggleHelp,
    enabled: isAuthenticated,
  });

  const handleLogout = useCallback(() => {
    const mode = getDashboardAuthMode();
    sessionStorage.removeItem('aether_auth');
    setIsAuthenticated(false);
    setUsername('');
    if (mode === 'cookie') {
      // Prefer local demo logout; OIDC/SAML/LDAP also clear aether_session via their handlers.
      window.location.assign('/api/auth/logout');
      return;
    }
    navigate('/', { replace: true });
    toast('Signed out of the dashboard', 'info');
  }, [navigate, toast]);

  const sequenceShortcuts = useMemo(
    () => [
      { sequence: ['g', 'd'] as [string, string], handler: () => handleNavigate('overview') },
      { sequence: ['g', 'a'] as [string, string], handler: () => handleNavigate('applications') },
      { sequence: ['g', 'w'] as [string, string], handler: () => handleNavigate('workloads') },
      { sequence: ['g', 'c'] as [string, string], handler: () => handleNavigate('clusters') },
      { sequence: ['g', 'h'] as [string, string], handler: () => handleNavigate('health') },
      { sequence: ['g', 'e'] as [string, string], handler: () => handleNavigate('events') },
      { sequence: ['g', 'b'] as [string, string], handler: () => handleNavigate('backups') },
      { sequence: ['g', 'u'] as [string, string], handler: () => handleNavigate('audit') },
      { sequence: ['g', 'm'] as [string, string], handler: () => handleNavigate('metrics') },
      { sequence: ['g', 'y'] as [string, string], handler: () => handleNavigate('policy') },
      { sequence: ['g', 's'] as [string, string], handler: () => handleNavigate('secrets') },
    ],
    [handleNavigate],
  );

  useSequenceShortcuts(sequenceShortcuts, isAuthenticated);

  const handleRefresh = useCallback(() => {
    setRefreshKey((k) => k + 1);
    setLastRefreshed(new Date());
    toast('Dashboard refreshed', 'info');
  }, [toast]);

  if (currentView === null) {
    return <Navigate to="/" replace />;
  }

  if (authBootstrapping) {
    return (
      <div className="app-shell min-h-screen flex flex-col items-center justify-center text-muted">
        <div className="w-10 h-10 rounded-xl border border-primary/30 bg-primary/10 mb-4 animate-pulse" aria-hidden />
        <p className="text-sm">Connecting to Aether…</p>
      </div>
    );
  }

  if (!isAuthenticated) {
    return (
      <LoginGate
        notice={loginNotice}
        onAuthenticated={(u) => {
          setUsername(u);
          setIsAuthenticated(true);
          setLoginNotice(undefined);
        }}
      />
    );
  }

  const hero = HERO_CONFIG[currentView];

  const shellClass = appShellClass();

  function renderPage() {
    switch (currentView) {
      case 'overview':
        return <OverviewPage username={username} onNavigate={handleNavigate} sseConnected={sseConnected} refreshKey={refreshKey} />;
      case 'fabric':
        return <FabricPage />;
      case 'migrations':
        return <MigrationsPage />;
      case 'observability':
        return <ObservabilityPage />;
      case 'labs':
        return <LabsPage />;
      case 'settings':
        return <SettingsPage />;
      case 'applications':
        return <ApplicationsPage refreshKey={refreshKey} />;
      case 'workloads':
        return (
          <WorkloadsPage
            key={selectedWorkloadFromPalette ?? ''}
            refreshKey={refreshKey}
            initialSelectedName={selectedWorkloadFromPalette}
            onClearInitialSelection={() => setSelectedWorkloadFromPalette(null)}
          />
        );
      case 'clusters':
        return <ClustersPage />;
      case 'clusters-browse':
        return <ClustersBrowsePage />;
      case 'clusters-network':
        return <ClustersNetworkPage />;
      case 'fleet':
        return <FleetPage />;
      case 'fleet-overview':
        return <FleetOverviewPage refreshKey={refreshKey} />;
      case 'fleet-edge':
        return <FleetEdgePage refreshKey={refreshKey} />;
      case 'fleet-placement':
        return <FleetPlacementPage refreshKey={refreshKey} />;
      case 'hosted':
        return <HostedPage refreshKey={refreshKey} />;
      case 'activity':
        return <ActivityMonitorPage />;
      case 'activity-cpu':
        return <ActivityCpuPage />;
      case 'activity-memory':
        return <ActivityMemoryPage />;
      case 'activity-restarts':
        return <ActivityRestartsPage />;
      case 'activity-errors':
        return <ActivityErrorsPage />;
      case 'security':
        return <SecurityCenterPage />;
      case 'security-overview':
        return <SecurityOverviewPage refreshKey={refreshKey} />;
      case 'security-copilot':
        return <SecurityCopilotPage refreshKey={refreshKey} />;
      case 'security-platform':
        return <SecurityPlatformPage refreshKey={refreshKey} />;
      case 'security-sbom':
        return <SecuritySbomPage refreshKey={refreshKey} />;
      case 'security-remediation':
        return <SecurityRemediationPage refreshKey={refreshKey} />;
      case 'security-threats':
        return <SecurityThreatsPage refreshKey={refreshKey} />;
      case 'helm':
        return <HelmCatalogPage refreshKey={refreshKey} />;
      case 'compose':
        return <ComposePage />;
      case 'ai':
        return <AIPage />;
      case 'ai-advisor':
        return <AiAdvisorPage refreshKey={refreshKey} />;
      case 'ai-designer':
        return <AiDesignerPage refreshKey={refreshKey} />;
      case 'ai-intent':
        return <AiIntentPage refreshKey={refreshKey} />;
      case 'ai-pipeline':
        return <AiPipelinePage refreshKey={refreshKey} />;
      case 'ai-recommend':
        return <AiRecommendPage refreshKey={refreshKey} />;
      case 'ai-optimize':
        return <AiOptimizePage refreshKey={refreshKey} />;
      case 'ai-analyze':
        return <AiAnalyzePage refreshKey={refreshKey} />;
      case 'zyra':
      case 'copilot':
        return <ZyraPage />;
      case 'ai-providers':
        return <AiProvidersPage refreshKey={refreshKey} />;
      case 'cost':
        return <CostPage />;
      case 'cost-intelligence':
        return <CostIntelligencePage refreshKey={refreshKey} />;
      case 'cost-finops':
        return <CostFinopsPage refreshKey={refreshKey} />;
      case 'cost-estimate':
        return <CostEstimatePage refreshKey={refreshKey} />;
      case 'affinity':
        return <AffinityPage />;
      case 'affinity-recommend':
        return <AffinityRecommendPage refreshKey={refreshKey} />;
      case 'affinity-matrix':
        return <AffinityMatrixPage refreshKey={refreshKey} />;
      case 'affinity-stats':
        return <AffinityStatsPage refreshKey={refreshKey} />;
      case 'drift':
        return <DriftPage refreshKey={refreshKey} />;
      case 'intelligence':
        return <IntelligencePage />;
      case 'intel-predictions':
        return <IntelPredictionsPage refreshKey={refreshKey} />;
      case 'intel-threats':
        return <IntelThreatsPage refreshKey={refreshKey} />;
      case 'intel-cost':
        return <IntelCostPage refreshKey={refreshKey} />;
      case 'intel-evolution':
        return <IntelEvolutionPage refreshKey={refreshKey} />;
      case 'intel-placement':
        return <IntelPlacementPage refreshKey={refreshKey} />;
      case 'policy':
        return <PolicyPage refreshKey={refreshKey} />;
      case 'scheduler':
        return <SchedulerPage refreshKey={refreshKey} />;
      case 'health':
        return <HealthPage refreshKey={refreshKey} />;
      case 'events':
        return <EventsPage refreshKey={refreshKey} />;
      case 'alerts':
        return <AlertsPage />;
      case 'alerts-channels':
        return <AlertsChannelsPage refreshKey={refreshKey} />;
      case 'alerts-rules':
        return <AlertsRulesPage refreshKey={refreshKey} />;
      case 'alerts-test':
        return <AlertsTestPage refreshKey={refreshKey} />;
      case 'alerts-queue':
        return <AlertsQueuePage refreshKey={refreshKey} />;
      case 'platform':
        return <PlatformPage />;
      case 'platform-recommendations':
        return <PlatformRecommendationsPage refreshKey={refreshKey} />;
      case 'platform-trust':
        return <PlatformTrustPage refreshKey={refreshKey} />;
      case 'platform-ecosystem':
        return <PlatformEcosystemPage refreshKey={refreshKey} />;
      case 'platform-runtime':
        return <PlatformRuntimePage refreshKey={refreshKey} />;
      case 'platform-cilium':
        return <PlatformCiliumPage refreshKey={refreshKey} />;
      case 'platform-observability':
        return <PlatformObservabilityPage refreshKey={refreshKey} />;
      case 'sla':
        return <SLAPage refreshKey={refreshKey} />;
      case 'deps':
        return <DepsPage refreshKey={refreshKey} />;
      case 'envs':
        return <EnvsPage refreshKey={refreshKey} />;
      case 'secrets':
        return <SecretsPage refreshKey={refreshKey} />;
      case 'backups':
        return <BackupsPage refreshKey={refreshKey} />;
      case 'storage':
        return <StoragePage refreshKey={refreshKey} />;
      case 'forge':
        return <ForgePage refreshKey={refreshKey} />;
      case 'templates':
        return <TemplatesPage refreshKey={refreshKey} />;
      case 'plugins':
        return <PluginsPage refreshKey={refreshKey} />;
      case 'rbac':
        return <RbacPage refreshKey={refreshKey} />;
      case 'audit':
        return <AuditPage refreshKey={refreshKey} />;
      case 'metrics':
        return <MetricsPage />;
      case 'metrics-summary':
        return <MetricsSummaryPage refreshKey={refreshKey} />;
      case 'metrics-chargeback':
        return <MetricsChargebackPage refreshKey={refreshKey} />;
      case 'metrics-observability':
        return <MetricsObservabilityPage refreshKey={refreshKey} />;
      case 'metrics-prometheus':
        return <MetricsPrometheusPage refreshKey={refreshKey} />;
      case 'gitops':
        return <GitOpsPage />;
      case 'gitops-center':
        return <GitopsCenterPage refreshKey={refreshKey} />;
      case 'gitops-agent':
        return <GitopsAgentPage refreshKey={refreshKey} />;
      case 'gitops-intent':
        return <GitopsIntentPage refreshKey={refreshKey} />;
      case 'gitops-sync':
        return <GitopsSyncPage refreshKey={refreshKey} />;
      case 'editor':
        return <EditorPage />;
      
      case 'obs-root-cause':
        return <ObsRootCausePage />;
      case 'obs-capacity-forecast':
        return <ObsCapacityForecastPage />;
      case 'obs-capacity-scale':
        return <ObsCapacityScalePage />;
      case 'obs-self-healing':
        return <ObsSelfHealingPage />;
      case 'obs-autonomous-sre':
        return <ObsAutonomousSrePage />;
      case 'obs-extensions':
        return <ObsExtensionsPage />;
      case 'obs-reliability':
        return <ObsReliabilityPage />;
      case 'settings-identity':
        return <SettingsIdentityPage />;
      case 'settings-nav':
        return <SettingsNavPage />;
      case 'settings-autonomous':
        return <SettingsAutonomousPage />;
      case 'settings-macos':
        return <SettingsMacosPage />;
      case 'labs-live':
        return <LabsLivePage />;
      case 'labs-graduation':
        return <LabsGraduationPage />;
      case 'labs-graph':
        return <LabsGraphPage />;
      case 'labs-platform':
        return <LabsPlatformPage />;
      case 'mig-planner':
        return <MigPlannerPage />;
      case 'mig-placement':
        return <MigPlacementPage />;
      case 'mig-replication':
        return <MigReplicationPage />;
      case 'mig-waves':
        return <MigWavesPage />;
      case 'fabric-twin':
        return <FabricTwinPage />;
      case 'fabric-topology':
        return <FabricTopologyPage />;
      case 'fabric-graph':
        return <FabricGraphPage />;
      case 'fabric-unified':
        return <FabricUnifiedPage />;

      case 'openapi':
        return <OpenApiPage refreshKey={refreshKey} />;
      default:
        return <OverviewPage username={username} onNavigate={handleNavigate} sseConnected={sseConnected} />;
    }
  }

  return (
    <AuthProvider enabled={isAuthenticated}>
    <WorkspaceProvider>
    <ServerCapabilitiesProvider>
      <DashboardShell
        shellClass={shellClass}
        currentView={currentView}
        heroTitle={hero.title}
        heroSubtitle={hero.subtitle}
        username={username}
        lastRefreshed={lastRefreshed}
        sseConnected={sseConnected}
        sseBannerVisible={sseBannerVisible}
        onNavigate={handleNavigate}
        onLogout={handleLogout}
        onRefresh={handleRefresh}
        onOpenCommandPalette={() => setCommandPaletteOpen(true)}
        commandPalette={
          <CommandPalette
            open={commandPaletteOpen}
            onClose={() => setCommandPaletteOpen(false)}
            onNavigate={handleNavigate}
            workloads={workloadNames}
            onSelectWorkload={(name) => setSelectedWorkloadFromPalette(name)}
            onRefresh={() => {
              setRefreshKey((k) => k + 1);
              setLastRefreshed(new Date());
            }}
            onLogout={handleLogout}
            onOpenHelp={(tab) => {
              setCommandPaletteOpen(false);
              openHelp(tab);
            }}
          />
        }
        onOpenHelp={openHelp}
        helpDialog={
          <HelpDialog
            open={helpOpen}
            tab={helpTab}
            onClose={() => setHelpOpen(false)}
            onTabChange={setHelpTab}
          />
        }
        toastContainer={<ToastContainer />}
        refreshKey={refreshKey}
        breadcrumbWorkload={breadcrumbWorkload}
      >
        <ErrorBoundary>
          <div className="page-frame">{renderPage()}</div>
        </ErrorBoundary>
      </DashboardShell>
    </ServerCapabilitiesProvider>
    </WorkspaceProvider>
    </AuthProvider>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="*" element={<AetherDashboard />} />
    </Routes>
  );
}
