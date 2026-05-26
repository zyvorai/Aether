// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router';
import type { AppView } from './types/api';
import DashboardShell from './components/DashboardShell';
import ErrorBoundary from './components/ErrorBoundary';
import Breadcrumb from './components/Breadcrumb';
import HelpDialog, { type HelpTab } from './components/HelpDialog';
import { useToast } from './components/Toast';
import { useEventStream } from './hooks/useEventStream';
import { useKeyboard } from './hooks/useKeyboard';
import { useSequenceShortcuts } from './hooks/useSequenceShortcut';
import { useTheme } from './contexts/ThemeContext';
import { appShellClass } from './utils/themeSurface';
import { ServerCapabilitiesProvider } from './contexts/ServerCapabilitiesContext';
import { AuthProvider } from './contexts/AuthContext';
import { pathToView, viewToPath } from './utils/dashboardRoutes';
import { HERO_CONFIG } from './utils/dashboardNav';
import { apiFetch, getDevBootstrapApiKey, DEFAULT_DASHBOARD_USERNAME, apiTryCookieSession, getDashboardAuthMode } from './utils/api';
import CommandPalette from './components/CommandPalette';
import { pushRecentView } from './utils/recentViews';
import LoginGate from './components/LoginGate';

// Page components — each is written by the pages agent
import OverviewPage from './components/pages/OverviewPage';
import WorkloadsPage from './components/pages/WorkloadsPage';
import ClustersPage from './components/pages/ClustersPage';
import ComposePage from './components/pages/ComposePage';
import AIPage from './components/pages/AIPage';
import CopilotPage from './components/pages/CopilotPage';
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
import TemplatesPage from './components/pages/TemplatesPage';
import PluginsPage from './components/pages/PluginsPage';
import RbacPage from './components/pages/RbacPage';
import AuditPage from './components/pages/AuditPage';
import MetricsPage from './components/pages/MetricsPage';
import GitOpsPage from './components/pages/GitOpsPage';
import EditorPage from './components/pages/EditorPage';
import ConfidentialPage from './components/pages/ConfidentialPage';
import IntelligencePage from './components/pages/IntelligencePage';
import FleetPage from './components/pages/FleetPage';

function AetherDashboard() {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const location = useLocation();

  const currentView = useMemo(() => pathToView(location.pathname), [location.pathname]);

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [authBootstrapping, setAuthBootstrapping] = useState(true);
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
        return;
      }

      setIsAuthenticated(false);
      setAuthBootstrapping(false);
    };
    void run();
  }, []);

  // Real-time updates via SSE
  const { connected: sseConnected } = useEventStream('', (event) => {
    if (event.type === 'workloadChanged' || event.type === 'healthUpdate') {
      setRefreshKey((k) => k + 1);
      setLastRefreshed(new Date());
    }
  }, isAuthenticated);

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
      window.location.assign('/api/auth/oidc/logout');
      return;
    }
    navigate('/', { replace: true });
    toast('Signed out of the dashboard', 'info');
  }, [navigate, toast]);

  const sequenceShortcuts = useMemo(
    () => [
      { sequence: ['g', 'd'] as [string, string], handler: () => handleNavigate('overview') },
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
      <div className="min-h-screen flex flex-col items-center justify-center bg-slate-950 text-slate-400">
        <div className="w-10 h-10 rounded-xl border border-aether/30 bg-aether/10 mb-4 animate-pulse" aria-hidden />
        <p className="text-sm">Connecting to Aether…</p>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <LoginGate onAuthenticated={(u) => { setUsername(u); setIsAuthenticated(true); }} />;
  }

  const hero = HERO_CONFIG[currentView];

  const shellClass = appShellClass(theme);

  function renderPage() {
    switch (currentView) {
      case 'overview':
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} sseConnected={sseConnected} />;
      case 'workloads':
        return (
          <WorkloadsPage
            key={`${refreshKey}-${selectedWorkloadFromPalette ?? ''}`}
            initialSelectedName={selectedWorkloadFromPalette}
            onClearInitialSelection={() => setSelectedWorkloadFromPalette(null)}
          />
        );
      case 'clusters':
        return <ClustersPage key={refreshKey} />;
      case 'fleet':
        return <FleetPage key={refreshKey} />;
      case 'compose':
        return <ComposePage key={refreshKey} />;
      case 'ai':
        return <AIPage key={refreshKey} />;
      case 'copilot':
        return <CopilotPage key={refreshKey} />;
      case 'cost':
        return <CostPage key={refreshKey} />;
      case 'affinity':
        return <AffinityPage key={refreshKey} />;
      case 'drift':
        return <DriftPage key={refreshKey} />;
      case 'intelligence':
        return <IntelligencePage key={refreshKey} />;
      case 'policy':
        return <PolicyPage key={refreshKey} />;
      case 'scheduler':
        return <SchedulerPage key={refreshKey} />;
      case 'health':
        return <HealthPage key={refreshKey} />;
      case 'events':
        return <EventsPage key={refreshKey} />;
      case 'alerts':
        return <AlertsPage key={refreshKey} />;
      case 'platform':
        return <PlatformPage key={refreshKey} />;
      case 'sla':
        return <SLAPage key={refreshKey} />;
      case 'deps':
        return <DepsPage key={refreshKey} />;
      case 'envs':
        return <EnvsPage key={refreshKey} />;
      case 'secrets':
        return <SecretsPage key={refreshKey} />;
      case 'backups':
        return <BackupsPage key={refreshKey} />;
      case 'templates':
        return <TemplatesPage key={refreshKey} />;
      case 'plugins':
        return <PluginsPage key={refreshKey} />;
      case 'rbac':
        return <RbacPage key={refreshKey} />;
      case 'audit':
        return <AuditPage key={refreshKey} />;
      case 'metrics':
        return <MetricsPage key={refreshKey} />;
      case 'gitops':
        return <GitOpsPage key={refreshKey} />;
      case 'editor':
        return <EditorPage key={refreshKey} />;
      case 'confidential':
        return <ConfidentialPage key={refreshKey} />;
      default:
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} sseConnected={sseConnected} />;
    }
  }

  return (
    <AuthProvider enabled={isAuthenticated}>
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
      >
        <ErrorBoundary>
          <Breadcrumb currentView={currentView} onNavigate={handleNavigate} />
          <div className="page-frame">{renderPage()}</div>
        </ErrorBoundary>
      </DashboardShell>
    </ServerCapabilitiesProvider>
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
