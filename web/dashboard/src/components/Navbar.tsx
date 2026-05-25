import { useState, useRef, useCallback, useEffect, useMemo, type ReactNode } from 'react';
import {
  Hexagon,
  LayoutDashboard,
  Container,
  Brain,
  DollarSign,
  Target,
  GitCompare,
  ShieldCheck,
  Shield,
  Settings,
  HeartPulse,
  Bell,
  BellRing,
  FileCheck,
  GitBranch,
  Layers,
  KeyRound,
  Archive,
  FileCode2,
  Puzzle,
  ClipboardList,
  BarChart3,
  RefreshCw,
  ChevronDown,
  Menu,
  X,
  Palette,
  FileText,
  Server,
  Command,
  LogOut,
  CircleHelp,
  Keyboard,
  Info,
  BookOpen,
  ExternalLink,
} from 'lucide-react';
import { ZYVOR_HELP } from '../config/zyvorHelp';
import type { HelpTab } from './HelpDialog';
import type { AppView } from '../types/api';
import { useTheme, type AppTheme } from '../contexts/ThemeContext';
import { useAuth } from '../contexts/AuthContext';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';
import { filterNavViews } from '../utils/navCapabilities';
import { getClusterContext } from '../utils/clusterContext';
import { getAuthToken, getDashboardAuthMode } from '../utils/api';
import PlatformHealthChip from './PlatformHealthChip';

function maskBearer(token: string | null): string {
  if (token === null || token.trim() === '') return 'Not set';
  return '••••••••';
}

function authSessionLabel(): { mode: string; preview: string } {
  const mode = getDashboardAuthMode();
  const token = getAuthToken();
  if (mode === 'cookie') {
    return { mode: 'Session', preview: 'OIDC cookie' };
  }
  if (mode === 'dev') {
    return { mode: 'Bearer', preview: 'Dev bootstrap' };
  }
  if (token !== null && token.trim() !== '') {
    return { mode: 'Bearer', preview: maskBearer(token) };
  }
  return { mode: 'Bearer', preview: 'Not set' };
}

interface NavbarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  username: string;
  onLogout: () => void;
  onRefresh: () => void;
  onOpenCommandPalette?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  lastRefreshed: Date;
  sseConnected?: boolean;
}

interface DropdownItem {
  label: string;
  view: AppView;
  icon: ReactNode;
}

interface DropdownGroup {
  label: string;
  items: DropdownItem[];
}

const intelligenceItems: DropdownItem[] = [
  { label: 'AI Engine', view: 'ai', icon: <Brain className="w-4 h-4" /> },
  { label: 'Cost Estimation', view: 'cost', icon: <DollarSign className="w-4 h-4" /> },
  { label: 'Runtime Affinity', view: 'affinity', icon: <Target className="w-4 h-4" /> },
  { label: 'Drift Detection', view: 'drift', icon: <GitCompare className="w-4 h-4" /> },
  { label: 'Policy Check', view: 'policy', icon: <ShieldCheck className="w-4 h-4" /> },
];

const operationsItems: DropdownItem[] = [
  { label: 'Cluster Browser', view: 'clusters', icon: <Container className="w-4 h-4" /> },
  { label: 'Compose Import', view: 'compose', icon: <Layers className="w-4 h-4" /> },
  { label: 'Visual Editor', view: 'editor', icon: <FileText className="w-4 h-4" /> },
  { label: 'Scheduler', view: 'scheduler', icon: <Settings className="w-4 h-4" /> },
  { label: 'Health Monitor', view: 'health', icon: <HeartPulse className="w-4 h-4" /> },
  { label: 'Events', view: 'events', icon: <Bell className="w-4 h-4" /> },
  { label: 'Platform & HA', view: 'platform', icon: <Server className="w-4 h-4" /> },
  { label: 'Alerts', view: 'alerts', icon: <BellRing className="w-4 h-4" /> },
  { label: 'SLA Compliance', view: 'sla', icon: <FileCheck className="w-4 h-4" /> },
  { label: 'Dependencies', view: 'deps', icon: <GitBranch className="w-4 h-4" /> },
  { label: 'Environments', view: 'envs', icon: <Layers className="w-4 h-4" /> },
];

const resourcesItems: DropdownItem[] = [
  { label: 'Secrets', view: 'secrets', icon: <KeyRound className="w-4 h-4" /> },
  { label: 'Backups', view: 'backups', icon: <Archive className="w-4 h-4" /> },
  { label: 'Templates', view: 'templates', icon: <FileCode2 className="w-4 h-4" /> },
  { label: 'Plugins', view: 'plugins', icon: <Puzzle className="w-4 h-4" /> },
  { label: 'Access Control', view: 'rbac', icon: <Shield className="w-4 h-4" /> },
  { label: 'Audit Trail', view: 'audit', icon: <ClipboardList className="w-4 h-4" /> },
  { label: 'GitOps', view: 'gitops', icon: <GitBranch className="w-4 h-4" /> },
  { label: 'Metrics', view: 'metrics', icon: <BarChart3 className="w-4 h-4" /> },
];

const dropdownGroups: DropdownGroup[] = [
  { label: 'Intelligence', items: intelligenceItems },
  { label: 'Operations', items: operationsItems },
  { label: 'Resources', items: resourcesItems },
];

const primaryItems: DropdownItem[] = [
  { label: 'Dashboard', view: 'overview', icon: <LayoutDashboard className="w-4 h-4" /> },
  { label: 'Workloads', view: 'workloads', icon: <Container className="w-4 h-4" /> },
];

const mobileNavGroups: DropdownGroup[] = [
  { label: 'Primary', items: primaryItems },
  ...dropdownGroups,
];

function Dropdown({
  group,
  isActive,
  currentView,
  onNavigate,
  theme,
}: {
  group: DropdownGroup;
  isActive: boolean;
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  theme: AppTheme;
}) {
  const [open, setOpen] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleEnter = useCallback(() => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setOpen(true), 150);
  }, []);

  const handleLeave = useCallback(() => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => setOpen(false), 100);
  }, []);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  return (
    <div
      ref={containerRef}
      className="relative"
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
    >
      <button
        className={`flex items-center gap-1 px-3 py-2 rounded-xl border text-sm font-medium transition-colors ${
          isActive
            ? 'nav-pill-active'
            : 'nav-pill'
        }`}
      >
        {group.label}
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div
          className={`absolute top-full left-0 mt-1 w-56 rounded-xl shadow-xl py-2 z-50 animate-scale-in border ${
            theme === 'light'
              ? 'bg-white border-slate-200 text-slate-800'
              : 'bg-zinc-900 border-zinc-700'
          }`}
        >
          {group.items.map((item) => (
            <button
              key={item.view}
              onClick={() => {
                onNavigate(item.view);
                setOpen(false);
              }}
              className={`w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors ${
                currentView === item.view
                  ? 'text-aether bg-aether/10'
                  : theme === 'light'
                    ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
              }`}
            >
              <span className={currentView === item.view ? 'text-aether' : theme === 'light' ? 'text-slate-400' : 'text-slate-500'}>
                {item.icon}
              </span>
              {item.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function useRelativeTime(date: Date): string {
  const [, setTick] = useState(0);
  useEffect(() => {
    const interval = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(interval);
  }, []);
  const seconds = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (seconds < 60) return `Updated ${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  return `Updated ${minutes}m ago`;
}

export default function Navbar({
  currentView,
  onNavigate,
  username,
  onLogout,
  onRefresh,
  onOpenCommandPalette,
  onOpenHelp,
  lastRefreshed,
  sseConnected,
}: NavbarProps) {
  const { theme, setTheme } = useTheme();
  const { role } = useAuth();
  const { capabilities, gitopsConfigured } = useServerCapabilities();
  const [clusterCtx, setClusterCtx] = useState<string | null>(() => getClusterContext());
  const [mobileOpen, setMobileOpen] = useState(false);
  const [helpMenuOpen, setHelpMenuOpen] = useState(false);
  const helpRef = useRef<HTMLDivElement>(null);
  const [spinning, setSpinning] = useState(false);
  const relativeTime = useRelativeTime(lastRefreshed);
  const { mode: authModeLabel, preview: bearerPreview } = authSessionLabel();

  useEffect(() => {
    const onCluster = () => setClusterCtx(getClusterContext());
    window.addEventListener('aether-cluster-context', onCluster);
    return () => window.removeEventListener('aether-cluster-context', onCluster);
  }, []);

  const navExtras = useMemo(() => ({ gitopsConfigured }), [gitopsConfigured]);
  const platform = capabilities?.platform ?? null;

  const filteredDropdownGroups = useMemo(
    () =>
      dropdownGroups.map((group) => ({
        ...group,
        items: filterNavViews(group.items, platform, navExtras),
      })),
    [platform, navExtras],
  );

  const filteredMobileNavGroups = useMemo(
    () =>
      mobileNavGroups.map((group) => ({
        ...group,
        items: filterNavViews(group.items, platform, navExtras),
      })),
    [platform, navExtras],
  );

  useEffect(() => {
    if (!helpMenuOpen) return;
    const onDocClick = (e: MouseEvent) => {
      if (helpRef.current && !helpRef.current.contains(e.target as Node)) {
        setHelpMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, [helpMenuOpen]);

  const handleRefreshClick = useCallback(() => {
    setSpinning(true);
    onRefresh();
    setTimeout(() => setSpinning(false), 600);
  }, [onRefresh]);

  const handleMobileNavigate = useCallback(
    (view: AppView) => {
      onNavigate(view);
      setMobileOpen(false);
    },
    [onNavigate],
  );

  return (
    <nav
      className={`sticky top-0 z-40 border-b ${
        theme === 'steel'
          ? 'border-[rgba(140,160,190,0.18)] bg-gradient-to-b from-[#0f141a] via-[#1a222d] to-[#0c1117] shadow-[inset_0_1px_0_rgba(255,255,255,0.06),0_10px_30px_rgba(0,0,0,0.45)]'
          : theme === 'light'
            ? 'bg-white/90 backdrop-blur-xl border-slate-200/90 shadow-sm'
            : 'navbar-blur border-slate-800/60'
      }`}
    >
      <div className="dash-content">
        <div className="flex items-center justify-between h-[72px]">
          {/* Left: Logo */}
          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={() => onNavigate('overview')}
              className="group flex items-center gap-2 transition-opacity hover:opacity-90"
            >
              <div className="relative flex h-10 w-10 items-center justify-center overflow-hidden rounded-2xl border border-aether/25 bg-aether/10 shadow-[0_0_0_1px_rgba(99,164,255,0.08),0_0_30px_rgba(99,164,255,0.08)] transition group-hover:border-aether/40">
                <div className="absolute inset-0 bg-gradient-to-br from-white/10 via-transparent to-aether/10" />
                <Hexagon className="w-5 h-5 text-aether" />
              </div>
              <div className="flex flex-col items-start">
                <span className="bg-gradient-to-r from-white to-slate-400 bg-clip-text text-lg font-semibold tracking-tight text-transparent">Aether</span>
                <span className="hidden lg:block text-[11px] uppercase tracking-[0.22em] text-slate-500">Universal Runtime Control Plane</span>
              </div>
            </button>
          </div>

          {/* Center: Navigation (desktop) */}
          <div className="hidden md:flex items-center gap-1">
            <button
              onClick={() => onNavigate('overview')}
              className={`flex items-center gap-2 px-3 py-2 rounded-xl border text-sm font-medium transition-colors ${
                currentView === 'overview'
                  ? 'nav-pill-active'
                  : 'nav-pill'
              }`}
            >
              <LayoutDashboard className="w-4 h-4" />
              Dashboard
            </button>

            <button
              onClick={() => onNavigate('workloads')}
              className={`flex items-center gap-2 px-3 py-2 rounded-xl border text-sm font-medium transition-colors ${
                currentView === 'workloads'
                  ? 'nav-pill-active'
                  : 'nav-pill'
              }`}
            >
              <Container className="w-4 h-4" />
              Workloads
            </button>

            {filteredDropdownGroups.map((group) => {
              if (group.items.length === 0) return null;
              const isActive = group.items.some((item) => item.view === currentView);
              return (
                <Dropdown
                  key={group.label}
                  group={group}
                  isActive={isActive}
                  currentView={currentView}
                  onNavigate={onNavigate}
                  theme={theme}
                />
              );
            })}
          </div>

          {/* Right: Actions */}
          <div className="flex items-center gap-1 sm:gap-2">
            <label className="hidden md:flex items-center gap-1 shrink-0 min-w-0" title="Theme">
              <Palette className={`w-3.5 h-3.5 shrink-0 ${theme === 'light' ? 'text-slate-500' : 'text-slate-500'}`} aria-hidden />
              <select
                aria-label="Theme"
                value={theme}
                onChange={(e) => setTheme(e.target.value as AppTheme)}
                className={`text-xs rounded-xl border px-1.5 sm:px-2 py-1.5 max-w-[6.5rem] sm:max-w-[7.5rem] cursor-pointer outline-none transition min-w-0 ${
                  theme === 'light'
                    ? 'bg-white border-slate-300 text-slate-800'
                    : theme === 'steel'
                      ? 'nav-steel-select text-[#d7dde5]'
                      : 'bg-slate-900/80 border-slate-600 text-slate-200'
                }`}
              >
                <option value="dark">Dark</option>
                <option value="steel">Steel</option>
                <option value="light">Light</option>
              </select>
            </label>
            <PlatformHealthChip sseConnected={sseConnected ?? false} />
            {clusterCtx ? (
              <span
                className="hidden lg:inline-flex items-center gap-1 rounded-lg border border-slate-700/80 bg-slate-900/60 px-2 py-1 text-[11px] text-slate-400 max-w-[10rem] truncate"
                title={`K8s context: ${clusterCtx}`}
              >
                <Container className="w-3 h-3 shrink-0 text-aether/80" />
                {clusterCtx}
              </span>
            ) : null}
            {onOpenHelp ? (
              <div className="relative hidden sm:block shrink-0" ref={helpRef}>
                <button
                  type="button"
                  onClick={() => setHelpMenuOpen((v) => !v)}
                  aria-expanded={helpMenuOpen}
                  aria-haspopup="menu"
                  className={`flex items-center gap-1 px-2 py-2 rounded-xl text-sm transition-colors ${
                    theme === 'light'
                      ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                      : helpMenuOpen
                        ? 'text-slate-100 bg-slate-800/80'
                        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                  }`}
                  title="Help (?)"
                  aria-label="Help menu"
                >
                  <CircleHelp className="w-4 h-4 shrink-0" aria-hidden />
                  <span className="hidden md:inline text-xs font-medium">Help</span>
                  <ChevronDown
                    className={`w-3 h-3 hidden md:block transition-transform ${helpMenuOpen ? 'rotate-180' : ''}`}
                    aria-hidden
                  />
                </button>
                {helpMenuOpen && (
                  <div
                    className={`absolute top-full right-0 mt-1.5 z-50 min-w-[12.5rem] rounded-xl py-1.5 shadow-xl animate-scale-in border ${
                      theme === 'light'
                        ? 'bg-white border-slate-200'
                        : 'bg-zinc-900 border-zinc-700'
                    }`}
                    role="menu"
                  >
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setHelpMenuOpen(false);
                        onOpenHelp('shortcuts');
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${
                        theme === 'light'
                          ? 'text-slate-700 hover:bg-slate-100'
                          : 'text-slate-300 hover:bg-slate-800/80'
                      }`}
                    >
                      <Keyboard className="w-4 h-4 shrink-0" aria-hidden />
                      Keyboard shortcuts
                      <kbd className="ml-auto text-[10px] px-1 py-0.5 rounded bg-slate-800/80 text-slate-500 font-mono">?</kbd>
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setHelpMenuOpen(false);
                        onOpenHelp('about');
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${
                        theme === 'light'
                          ? 'text-slate-700 hover:bg-slate-100'
                          : 'text-slate-300 hover:bg-slate-800/80'
                      }`}
                    >
                      <Info className="w-4 h-4 shrink-0" aria-hidden />
                      About
                    </button>
                    <a
                      role="menuitem"
                      href={ZYVOR_HELP.docs}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => setHelpMenuOpen(false)}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${
                        theme === 'light'
                          ? 'text-slate-700 hover:bg-slate-100'
                          : 'text-slate-300 hover:bg-slate-800/80'
                      }`}
                    >
                      <BookOpen className="w-4 h-4 shrink-0" aria-hidden />
                      Help &amp; documentation
                      <ExternalLink className="w-3 h-3 ml-auto opacity-60" aria-hidden />
                    </a>
                    <a
                      role="menuitem"
                      href={ZYVOR_HELP.contact}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => setHelpMenuOpen(false)}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${
                        theme === 'light'
                          ? 'text-slate-700 hover:bg-slate-100'
                          : 'text-slate-300 hover:bg-slate-800/80'
                      }`}
                    >
                      <ExternalLink className="w-4 h-4 shrink-0" aria-hidden />
                      Contact support
                    </a>
                    <div className={`px-3 py-2 border-t text-[11px] ${
                      theme === 'light' ? 'border-slate-200' : 'border-slate-700/50'
                    }`}>
                      <a
                        href={ZYVOR_HELP.platform}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-orange-400 hover:text-orange-300"
                        onClick={() => setHelpMenuOpen(false)}
                      >
                        zyvor.dev · © 2026
                      </a>
                    </div>
                  </div>
                )}
              </div>
            ) : null}
            <span
              className={`sm:hidden inline-block w-2 h-2 rounded-full ${sseConnected ? 'bg-emerald-400 platform-pulse' : 'bg-red-400'}`}
              title={sseConnected ? 'SSE Connected' : 'SSE Disconnected'}
            />
            <span className={`hidden sm:inline text-xs ${theme === 'light' ? 'text-slate-500' : 'text-slate-500'}`}>{relativeTime}</span>
            <button
              onClick={handleRefreshClick}
              className={`p-2 rounded-xl transition-colors ${
                theme === 'light'
                  ? 'text-slate-500 hover:text-slate-900 hover:bg-slate-100'
                  : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
              }`}
              title="Refresh"
            >
              <RefreshCw className={`w-4 h-4${spinning ? ' animate-spin' : ''}`} />
            </button>

            <div className="hidden lg:flex shrink-0 items-center gap-2 px-3 py-2 rounded-xl surface-panel-soft metric-glow">
              <Shield className="w-4 h-4 shrink-0 text-aether" />
              <div className="flex min-w-0 flex-col gap-0.5 leading-tight">
                <span className={`text-[10px] uppercase tracking-[0.14em] ${theme === 'light' ? 'text-slate-500' : 'text-slate-500'}`}>{authModeLabel}</span>
                <span
                  className={`whitespace-nowrap text-sm font-medium font-mono tracking-tight ${theme === 'light' ? 'text-slate-800' : 'text-slate-200'}`}
                  title={bearerPreview}
                >
                  {bearerPreview}
                </span>
              </div>
            </div>

            <div className="hidden sm:flex items-center gap-2 px-3 py-2 rounded-xl surface-panel-soft">
              <div className="w-2 h-2 rounded-full bg-emerald-400" />
              <span className={`text-sm ${theme === 'light' ? 'text-slate-700' : 'text-slate-300'}`}>{username}</span>
              <span className="rounded-full border border-aether/30 bg-aether/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-aether">
                {role}
              </span>
            </div>
            <button
              type="button"
              onClick={onLogout}
              className={`hidden sm:inline-flex px-3 py-2 rounded-xl text-sm transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40 ${
                theme === 'light'
                  ? 'text-slate-600 hover:bg-slate-100'
                  : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
              }`}
            >
              Sign out
            </button>{/* Mobile hamburger */}
            <button
              onClick={() => setMobileOpen((v) => !v)}
              className={`md:hidden p-2 rounded-xl transition-colors ${
                theme === 'light'
                  ? 'text-slate-500 hover:text-slate-900 hover:bg-slate-100'
                  : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
              }`}
              aria-label="Toggle menu"
            >
              {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile menu overlay */}
      {mobileOpen && (
        <div
          className={`md:hidden border-t animate-fade-in backdrop-blur-lg max-h-[min(70vh,32rem)] overflow-y-auto ${
            theme === 'light' ? 'border-slate-200 bg-white/95' : 'border-slate-800/60 bg-slate-950/95'
          }`}
        >
          <div className="dash-content py-3 space-y-4">
            {onOpenCommandPalette ? (
              <button
                type="button"
                onClick={() => {
                  onOpenCommandPalette();
                  setMobileOpen(false);
                }}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium border ${
                  theme === 'light'
                    ? 'border-slate-200 bg-slate-50 text-slate-800 hover:bg-slate-100'
                    : 'border-aether/30 bg-aether/10 text-aether hover:bg-aether/15'
                }`}
              >
                <Command className="w-4 h-4" />
                Command palette
                <kbd className="ml-auto text-[10px] opacity-70">⌘K</kbd>
              </button>
            ) : null}

            <div className="rounded-2xl surface-panel-soft px-4 py-3">
              <div className="text-[10px] uppercase tracking-[0.2em] text-slate-500">{authModeLabel}</div>
              <div className={`mt-1 text-sm font-medium font-mono tracking-tight ${theme === 'light' ? 'text-slate-800' : 'text-slate-200'}`}>
                {bearerPreview}
              </div>
              <div className={`mt-2 text-sm ${theme === 'light' ? 'text-slate-600' : 'text-slate-400'}`}>{username}</div>
            </div>

            {filteredMobileNavGroups.map((group) => (
              group.items.length === 0 ? null : (
              <div key={group.label}>
                <div className="px-4 pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-slate-500">
                  {group.label}
                </div>
                <div className="space-y-0.5">
                  {group.items.map((item) => (
                    <button
                      key={item.view}
                      type="button"
                      onClick={() => handleMobileNavigate(item.view)}
                      className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                        currentView === item.view
                          ? 'text-aether bg-aether/10'
                          : theme === 'light'
                            ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                            : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                      }`}
                    >
                      <span className={currentView === item.view ? 'text-aether' : theme === 'light' ? 'text-slate-400' : 'text-slate-500'}>
                        {item.icon}
                      </span>
                      {item.label}
                    </button>
                  ))}
                </div>
              </div>
              ))
            )}

            {onOpenHelp ? (
              <div className="px-2">
                <div className="px-2 pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-slate-500">Help</div>
                <div className="space-y-0.5">
                  <button
                    type="button"
                    onClick={() => {
                      onOpenHelp('shortcuts');
                      setMobileOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                      theme === 'light'
                        ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                    }`}
                  >
                    <Keyboard className="w-4 h-4" />
                    Keyboard shortcuts
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      onOpenHelp('about');
                      setMobileOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                      theme === 'light'
                        ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                    }`}
                  >
                    <Info className="w-4 h-4" />
                    About
                  </button>
                  <a
                    href={ZYVOR_HELP.docs}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={() => setMobileOpen(false)}
                    className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                      theme === 'light'
                        ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                    }`}
                  >
                    <BookOpen className="w-4 h-4" />
                    Help &amp; documentation
                  </a>
                  <a
                    href={ZYVOR_HELP.contact}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={() => setMobileOpen(false)}
                    className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                      theme === 'light'
                        ? 'text-slate-600 hover:text-slate-900 hover:bg-slate-100'
                        : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80'
                    }`}
                  >
                    <ExternalLink className="w-4 h-4" />
                    Contact support
                  </a>
                  <div
                    className={`px-4 py-2 text-[11px] ${
                      theme === 'light' ? 'text-slate-500' : 'text-slate-500'
                    }`}
                  >
                    <a
                      href={ZYVOR_HELP.platform}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => setMobileOpen(false)}
                      className="text-orange-400 hover:text-orange-300"
                    >
                      zyvor.dev · © 2026
                    </a>
                  </div>
                </div>
              </div>
            ) : null}

            <div className="flex flex-wrap items-center gap-2 px-2 pt-2 border-t border-slate-800/60">
              <label className="flex items-center gap-2 flex-1 min-w-[8rem]">
                <Palette className="w-4 h-4 text-slate-500 shrink-0" aria-hidden />
                <select
                  aria-label="Theme"
                  value={theme}
                  onChange={(e) => setTheme(e.target.value as AppTheme)}
                  className={`flex-1 text-xs rounded-xl border px-2 py-2 cursor-pointer outline-none ${
                    theme === 'light'
                      ? 'bg-white border-slate-300 text-slate-800'
                      : theme === 'steel'
                        ? 'nav-steel-select text-[#d7dde5]'
                        : 'bg-slate-900/80 border-slate-600 text-slate-200'
                  }`}
                >
                  <option value="dark">Dark</option>
                  <option value="steel">Steel</option>
                  <option value="light">Light</option>
                </select>
              </label>
              <button
                type="button"
                onClick={() => {
                  onLogout();
                  setMobileOpen(false);
                }}
                className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm transition-colors ${
                  theme === 'light'
                    ? 'text-slate-600 hover:bg-slate-100 border border-slate-200'
                    : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800/80 border border-slate-700/60'
                }`}
              >
                <LogOut className="w-4 h-4" />
                Sign out
              </button>
            </div>
          </div>
        </div>
      )}
    </nav>
  );
}
