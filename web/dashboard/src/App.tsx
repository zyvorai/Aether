import { useState, useEffect, useCallback } from 'react';
import type { AppView } from './types/api';
import Login from './components/Login';
import Navbar from './components/Navbar';
import Hero from './components/Hero';
import Footer from './components/Footer';
import ErrorBoundary from './components/ErrorBoundary';
import { useToast } from './components/Toast';
import { useEventStream } from './hooks/useEventStream';
import { useKeyboard } from './hooks/useKeyboard';
import { apiFetch } from './utils/api';
import CommandPalette from './components/CommandPalette';

// Page components — each is written by the pages agent
import OverviewPage from './components/pages/OverviewPage';
import WorkloadsPage from './components/pages/WorkloadsPage';
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
import AuditPage from './components/pages/AuditPage';
import MetricsPage from './components/pages/MetricsPage';

const heroConfig: Record<AppView, { title: string; subtitle: string }> = {
  overview: { title: 'Dashboard', subtitle: 'Real-time overview of your universal runtime control plane' },
  workloads: { title: 'Workloads', subtitle: 'Manage and monitor deployed workloads across all runtimes' },
  ai: { title: 'AI Engine', subtitle: 'Intelligent runtime scoring, scaling advice, and migration planning' },
  cost: { title: 'Cost Estimation', subtitle: 'Analyze and forecast infrastructure costs across providers' },
  affinity: { title: 'Runtime Affinity', subtitle: 'Historical performance scores and deployment success rates' },
  drift: { title: 'Drift Detection', subtitle: 'Detect and reconcile configuration drift in live workloads' },
  policy: { title: 'Policy Check', subtitle: 'Evaluate workloads against security and compliance policies' },
  scheduler: { title: 'Scheduler', subtitle: 'Runtime utilization, placement decisions, and optimization' },
  health: { title: 'Health Monitor', subtitle: 'Real-time health checks, circuit breakers, and restart tracking' },
  events: { title: 'Events', subtitle: 'System events, alerts, and notifications across all runtimes' },
  sla: { title: 'SLA Compliance', subtitle: 'Track uptime targets, latency budgets, and error rate limits' },
  deps: { title: 'Dependencies', subtitle: 'Workload dependency graph, startup order, and cycle detection' },
  envs: { title: 'Environments', subtitle: 'Manage environment tiers, variables, and workload assignments' },
  secrets: { title: 'Secrets', subtitle: 'Encrypted secrets management with rotation policies' },
  backups: { title: 'Backups', subtitle: 'State snapshots, restore points, and backup history' },
  templates: { title: 'Templates', subtitle: 'Reusable workload templates for rapid deployment' },
  plugins: { title: 'Plugins', subtitle: 'Installed runtime plugins and their capabilities' },
  audit: { title: 'Audit Trail', subtitle: 'Immutable log of all actions with integrity verification' },
  metrics: { title: 'Metrics', subtitle: 'System performance metrics and resource utilization trends' },
};

export default function App() {
  const [currentView, setCurrentView] = useState<AppView>('overview');
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [username, setUsername] = useState('');
  const [refreshKey, setRefreshKey] = useState(0);
  const [lastRefreshed, setLastRefreshed] = useState<Date>(new Date());
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [workloadNames, setWorkloadNames] = useState<string[]>([]);
  const { toast, ToastContainer } = useToast();

  // Check session on mount
  useEffect(() => {
    const stored = sessionStorage.getItem('aether_auth');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        if (parsed.authenticated && parsed.username) {
          setIsAuthenticated(true);
          setUsername(parsed.username);
        }
      } catch {
        sessionStorage.removeItem('aether_auth');
      }
    }
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
    apiFetch<Array<{ name: string }>>('/workloads').then(r => {
      if (r) setWorkloadNames(r.map((w) => w.name));
    });
  }, [isAuthenticated, refreshKey]);

  // Keyboard shortcuts
  useKeyboard({
    onRefresh: () => { setRefreshKey(k => k + 1); setLastRefreshed(new Date()); },
    onCommandPalette: () => setCommandPaletteOpen(o => !o),
    enabled: isAuthenticated,
  });

  const handleLogin = useCallback((user: string) => {
    setIsAuthenticated(true);
    setUsername(user);
    sessionStorage.setItem('aether_auth', JSON.stringify({ authenticated: true, username: user }));
    toast('Signed in successfully', 'success');
  }, [toast]);

  const handleLogout = useCallback(() => {
    setIsAuthenticated(false);
    setUsername('');
    setCurrentView('overview');
    sessionStorage.removeItem('aether_auth');
  }, []);

  const handleNavigate = useCallback((view: AppView) => {
    setCurrentView(view);
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }, []);

  const handleRefresh = useCallback(() => {
    setRefreshKey((k) => k + 1);
    setLastRefreshed(new Date());
    toast('Dashboard refreshed', 'info');
  }, [toast]);

  if (!isAuthenticated) {
    return <Login onLogin={handleLogin} />;
  }

  const hero = heroConfig[currentView];

  function renderPage() {
    switch (currentView) {
      case 'overview':
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} />;
      case 'workloads':
        return <WorkloadsPage key={refreshKey} />;
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
      case 'audit':
        return <AuditPage key={refreshKey} />;
      case 'metrics':
        return <MetricsPage key={refreshKey} />;
      default:
        return <OverviewPage key={refreshKey} onNavigate={handleNavigate} />;
    }
  }

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
      <Navbar
        currentView={currentView}
        onNavigate={handleNavigate}
        username={username}
        onLogout={handleLogout}
        onRefresh={handleRefresh}
        lastRefreshed={lastRefreshed}
        sseConnected={sseConnected}
      />
      <Hero title={hero.title} subtitle={hero.subtitle} />
      <main className="flex-1 max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-8">
        <ErrorBoundary>
          {renderPage()}
        </ErrorBoundary>
      </main>
      <Footer />
      <CommandPalette
        open={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
        onNavigate={setCurrentView}
        workloads={workloadNames}
        onRefresh={() => { setRefreshKey(k => k + 1); setLastRefreshed(new Date()); }}
      />
      <ToastContainer />
    </div>
  );
}
