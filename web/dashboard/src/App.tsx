import { useState, useEffect, useCallback, useMemo } from 'react';
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router';
import type { AppView } from './types/api';
import DashboardShell from './components/DashboardShell';
import ErrorBoundary from './components/ErrorBoundary';
import Breadcrumb from './components/Breadcrumb';
import ShortcutsHelp from './components/ShortcutsHelp';
import { useToast } from './components/Toast';
import { useEventStream } from './hooks/useEventStream';
import { useKeyboard } from './hooks/useKeyboard';
import { useSequenceShortcuts } from './hooks/useSequenceShortcut';
import { useTheme } from './contexts/ThemeContext';
import { ServerCapabilitiesProvider } from './contexts/ServerCapabilitiesContext';
import { pathToView, viewToPath } from './utils/dashboardRoutes';
import { apiFetch, getDevBootstrapApiKey, DEFAULT_DASHBOARD_USERNAME, apiTryCookieSession, getDashboardAuthMode } from './utils/api';
import CommandPalette from './components/CommandPalette';
import LoginGate from './components/LoginGate';

// Page components — each is written by the pages agent
import OverviewPage from './components/pages/OverviewPage';
import WorkloadsPage from './components/pages/WorkloadsPage';
import ClustersPage from './components/pages/ClustersPage';
import ComposePage from './components/pages/ComposePage';
import AIPage from './components/pages/AIPage';
import CostPage from './components/pages/CostPage';
import AffinityPage from './components/pages/AffinityPage';
import DriftPage from './components/pages/DriftPage';
import PolicyPage from './components/pages/PolicyPage';
import SchedulerPage from './components/pages/SchedulerPage';
import HealthPage from './components/pages/HealthPage';
import EventsPage from './components/pages/EventsPage';
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

const heroConfig: Record<AppView, { title: string; subtitle: string }> = {
  overview: { title: 'Dashboard', subtitle: '' },
  workloads: { title: 'Workloads', subtitle: '' },
  clusters: { title: 'Cluster Browser', subtitle: '' },
  compose: { title: 'Compose Import', subtitle: '' },
  ai: { title: 'AI Engine', subtitle: '' },
  cost: { title: 'Cost Estimation', subtitle: '' },
  affinity: { title: 'Runtime Affinity', subtitle: '' },
  drift: { title: 'Drift Detection', subtitle: '' },
  policy: { title: 'Policy Check', subtitle: '' },
  scheduler: { title: 'Scheduler', subtitle: '' },
  health: { title: 'Health Monitor', subtitle: '' },
  events: { title: 'Events', subtitle: '' },
  sla: { title: 'SLA Compliance', subtitle: '' },
  deps: { title: 'Dependencies', subtitle: '' },
  envs: { title: 'Environments', subtitle: '' },
  secrets: { title: 'Secrets', subtitle: '' },
  backups: { title: 'Backups', subtitle: '' },
  templates: { title: 'Templates', subtitle: '' },
  plugins: { title: 'Plugins', subtitle: '' },
  rbac: { title: 'Access Control', subtitle: '' },
  audit: { title: 'Audit Trail', subtitle: '' },
  metrics: { title: 'Metrics', subtitle: '' },
};

function AetherDashboard() {
  const { theme } = useTheme();
  const navigate = useNavigate();
  const location = useLocation();

  const currentView = useMemo(() => pathToView(location.pathname), [location.pathname]);

  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [username, setUsername] = useState('');
  const [refreshKey, setRefreshKey] = useState(0);
  const [lastRefreshed, setLastRefreshed] = useState<Date>(new Date());
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [workloadNames, setWorkloadNames] = useState<string[]>([]);
  const { toast, ToastContainer } = useToast();

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
        return;
      }

      setIsAuthenticated(false);
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

  // Keyboard shortcuts
  useKeyboard({
    onRefresh: () => {
      setRefreshKey((k) => k + 1);
      setLastRefreshed(new Date());
    },
    onCommandPalette: () => setCommandPaletteOpen((o) => !o),
    onShortcutsHelp: () => setShortcutsOpen((o) => !o),
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

  if (!isAuthenticated) {
    return <LoginGate onAuthenticated={(u) => { setUsername(u); setIsAuthenticated(true); }} />;
  }

  const hero = heroConfig[currentView];

  const shellClass =
    theme === 'steel'
      ? 'dashboard-steel min-h-screen flex flex-col text-[#d7dde5] steel-grid'
      : theme === 'light'
        ? 'min-h-screen flex flex-col text-slate-900'
        : 'app-shell min-h-screen flex flex-col text-slate-100 steel-grid';

  function renderPage() {
    switch (currentView) {
      case 'overview':
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} sseConnected={sseConnected} />;
      case 'workloads':
        return <WorkloadsPage key={refreshKey} />;
      case 'clusters':
        return <ClustersPage key={refreshKey} />;
      case 'compose':
        return <ComposePage key={refreshKey} />;
      case 'ai':
        return <AIPage key={refreshKey} />;
      case 'cost':
        return <CostPage key={refreshKey} />;
      case 'affinity':
        return <AffinityPage key={refreshKey} />;
      case 'drift':
        return <DriftPage key={refreshKey} />;
      case 'policy':
        return <PolicyPage key={refreshKey} />;
      case 'scheduler':
        return <SchedulerPage key={refreshKey} />;
      case 'health':
        return <HealthPage key={refreshKey} />;
      case 'events':
        return <EventsPage key={refreshKey} />;
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
      default:
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} sseConnected={sseConnected} />;
    }
  }

  return (
    <ServerCapabilitiesProvider>
      <DashboardShell
        shellClass={shellClass}
        currentView={currentView}
        heroTitle={hero.title}
        heroSubtitle={hero.subtitle}
        username={username}
        lastRefreshed={lastRefreshed}
        sseConnected={sseConnected}
        onNavigate={handleNavigate}
        onLogout={handleLogout}
        onRefresh={handleRefresh}
        commandPalette={
          <CommandPalette
            open={commandPaletteOpen}
            onClose={() => setCommandPaletteOpen(false)}
            onNavigate={handleNavigate}
            workloads={workloadNames}
            onRefresh={() => {
              setRefreshKey((k) => k + 1);
              setLastRefreshed(new Date());
            }}
          />
        }
        shortcutsHelp={shortcutsOpen ? <ShortcutsHelp onClose={() => setShortcutsOpen(false)} /> : null}
        toastContainer={<ToastContainer />}
      >
        <ErrorBoundary>
          <Breadcrumb currentView={currentView} onNavigate={handleNavigate} />
          <div className="page-frame">{renderPage()}</div>
        </ErrorBoundary>
      </DashboardShell>
    </ServerCapabilitiesProvider>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="*" element={<AetherDashboard />} />
    </Routes>
  );
}
