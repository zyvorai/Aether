// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
  Lock,
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
  HardDrive,
  Cpu,
  FileCode2,
  Puzzle,
  ClipboardList,
  BarChart3,
  RefreshCw,
  Rocket,
  ChevronDown,
  Menu,
  X,
  Sun,
  Moon,
  FileText,
  Server,
  Command,
  LogOut,
  UserCircle,
  CircleHelp,
  Keyboard,
  Info,
  BookOpen,
  ExternalLink,
  Sparkles,
  Globe,
  Boxes,
  Clock,
} from 'lucide-react';
import { ZYVOR_HELP } from '../config/zyvorHelp';
import type { HelpTab } from './HelpDialog';
import type { AppView } from '../types/api';
import { useProView } from '../hooks/useProView';
import { useTheme, ACCENT_OPTIONS } from '../contexts/ThemeContext';
import { dropdownItemClass, dropdownSurfaceClass, navbarShellClass } from '../utils/themeSurface';
import { useAuth } from '../contexts/AuthContext';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';
import { filterNavViews, partitionNavViews, type AnnotatedNavItem } from '../utils/navCapabilities';
import { AI_OS_NAV, isAiOsNavActive } from '../utils/aiOsNav';
import { getClusterContext } from '../utils/clusterContext';
import { getRecentViews } from '../utils/recentViews';
import { getViewMeta, NAV_GROUP_COLOR, type NavGroup } from '../utils/dashboardNav';
import { getAuthToken, getDashboardAuthMode } from '../utils/api';
import { isMacOSShell } from '../utils/macosBridge';
import PlatformHealthChip from './PlatformHealthChip';
import TenantSwitcher from './TenantSwitcher';

function maskBearer(token: string | null): string {
  if (token === null || token.trim() === '') return 'Not set';
  return '••••••••';
}

function authSessionLabel(): { mode: string; preview: string } {
  const mode = getDashboardAuthMode();
  const token = getAuthToken();
  if (mode === 'cookie') {
    return { mode: 'Session', preview: 'Cookie session' };
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
  { label: 'Intelligence Layer', view: 'intelligence', icon: <Sparkles className="w-4 h-4" /> },
  { label: 'Cost Estimation', view: 'cost', icon: <DollarSign className="w-4 h-4" /> },
  { label: 'Runtime Affinity', view: 'affinity', icon: <Target className="w-4 h-4" /> },
  { label: 'Drift Detection', view: 'drift', icon: <GitCompare className="w-4 h-4" /> },
  { label: 'Policy Check', view: 'policy', icon: <ShieldCheck className="w-4 h-4" /> },
  { label: 'Security Center', view: 'security', icon: <Shield className="w-4 h-4" /> },
  { label: 'Confidential Computing', view: 'confidential', icon: <Lock className="w-4 h-4" /> },
];

const operationsItems: DropdownItem[] = [
  { label: 'Cluster Browser', view: 'clusters', icon: <Container className="w-4 h-4" /> },
  { label: 'Fleet Overview', view: 'fleet', icon: <Globe className="w-4 h-4" /> },
  { label: 'Activity Monitor', view: 'activity', icon: <BarChart3 className="w-4 h-4" /> },
  { label: 'Helm App Store', view: 'helm', icon: <FileCode2 className="w-4 h-4" /> },
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
  { label: 'Storage', view: 'storage', icon: <HardDrive className="w-4 h-4" /> },
  { label: 'GPU / Forge', view: 'forge', icon: <Cpu className="w-4 h-4" /> },
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

const cloudOsNavItems: DropdownItem[] = [
  { label: 'Applications', view: 'applications', icon: <Boxes className="w-4 h-4" /> },
  { label: 'Health', view: 'health', icon: <HeartPulse className="w-4 h-4" /> },
  { label: 'Logs', view: 'events', icon: <Bell className="w-4 h-4" /> },
  { label: 'Deployments', view: 'workloads', icon: <Container className="w-4 h-4" /> },
  { label: 'Storage', view: 'storage', icon: <HardDrive className="w-4 h-4" /> },
  { label: 'GPU / Forge', view: 'forge', icon: <Cpu className="w-4 h-4" /> },
  { label: 'Network', view: 'clusters', icon: <Globe className="w-4 h-4" /> },
  { label: 'Backups', view: 'backups', icon: <Archive className="w-4 h-4" /> },
  { label: 'Settings', view: 'platform', icon: <Settings className="w-4 h-4" /> },
];

const primaryItems: DropdownItem[] = [
  { label: 'Dashboard', view: 'overview', icon: <LayoutDashboard className="w-4 h-4" /> },
  { label: 'Applications', view: 'applications', icon: <Boxes className="w-4 h-4" /> },
  { label: 'Workloads', view: 'workloads', icon: <Container className="w-4 h-4" /> },
];

const mobileNavGroups: DropdownGroup[] = [
  { label: 'Primary', items: primaryItems },
  ...dropdownGroups,
];

function Dropdown({
  group,
  ready,
  setup,
  isActive,
  currentView,
  onNavigate,
}: {
  group: DropdownGroup;
  ready: AnnotatedNavItem<DropdownItem>[];
  setup: AnnotatedNavItem<DropdownItem>[];
  isActive: boolean;
  currentView: AppView;
  onNavigate: (view: AppView) => void;
}) {
  const [open, setOpen] = useState(false);
  const hoverRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const clearHoverTimer = useCallback(() => {
    if (hoverRef.current) {
      clearTimeout(hoverRef.current);
      hoverRef.current = null;
    }
  }, []);

  const scheduleClose = useCallback(() => {
    clearHoverTimer();
    hoverRef.current = setTimeout(() => setOpen(false), 120);
  }, [clearHoverTimer]);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  useEffect(() => () => clearHoverTimer(), [clearHoverTimer]);

  return (
    <div
      ref={containerRef}
      className="relative shrink-0"
      onMouseEnter={() => {
        clearHoverTimer();
        hoverRef.current = setTimeout(() => setOpen(true), 80);
      }}
      onMouseLeave={scheduleClose}
    >
      <button
        type="button"
        aria-expanded={open}
        aria-haspopup="menu"
        onClick={() => {
          clearHoverTimer();
          setOpen((v) => !v);
        }}
        className={`flex shrink-0 items-center gap-1 rounded-xl border px-2.5 py-1.5 text-sm font-medium transition-colors lg:px-3 lg:py-2 ${
          isActive || open ? 'nav-pill-active' : 'nav-pill'
        }`}
      >
        <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${NAV_GROUP_COLOR[group.label.toLowerCase() as NavGroup] ?? 'bg-ink-3'}`} aria-hidden />
        {group.label}
        <ChevronDown className={`h-3.5 w-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div
          className="absolute left-0 top-full z-[60] pt-1"
          onMouseEnter={clearHoverTimer}
          onMouseLeave={scheduleClose}
        >
          <div
            role="menu"
            className={`w-56 animate-scale-in rounded-xl border py-2 shadow-xl ${dropdownSurfaceClass()}`}
          >
            {ready.map((item) => (
              <button
                key={item.view}
                type="button"
                role="menuitem"
                onClick={() => {
                  onNavigate(item.view);
                  setOpen(false);
                }}
                className={`flex w-full items-center gap-3 px-4 py-2.5 text-sm transition-colors ${dropdownItemClass(currentView === item.view)}`}
              >
                <span className={currentView === item.view ? 'text-brand' : 'text-ink-3'}>
                  {item.icon}
                </span>
                {item.label}
              </button>
            ))}
            {setup.length > 0 ? (
              <>
                <div className="mx-3 my-1 glass-divider-t/60/80" />
                <p className="px-4 py-1 text-[10px] uppercase tracking-wider text-ink-3">Setup required</p>
                {setup.map((item) => (
                  <button
                    key={`setup-${item.view}`}
                    type="button"
                    role="menuitem"
                    title={item.visibility.setupHint ?? 'Configure on Platform & HA'}
                    onClick={() => {
                      onNavigate('platform');
                      setOpen(false);
                    }}
                    className={`flex w-full items-center gap-3 px-4 py-2.5 text-sm opacity-50 transition-colors hover:opacity-80 ${dropdownItemClass(false)}`}
                  >
                    <span className="text-ink-3">{item.icon}</span>
                    <span className="flex flex-col items-start gap-0.5 text-left">
                      <span>{item.label}</span>
                      {item.visibility.setupHint ? (
                        <span className="text-[10px] leading-tight text-ink-3">{item.visibility.setupHint}</span>
                      ) : null}
                    </span>
                  </button>
                ))}
              </>
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}

function AccountMenu({
  username,
  role,
  authModeLabel,
  bearerPreview,
  clusterCtx,
  onLogout,
}: {
  username: string;
  role: string;
  authModeLabel: string;
  bearerPreview: string;
  clusterCtx: string | null;
  onLogout: () => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, [open]);

  return (
    <div className="relative shrink-0" ref={ref}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        aria-haspopup="menu"
        className={`flex items-center gap-1.5 rounded-xl border px-2 py-1.5 text-sm transition-colors sm:px-2.5 ${
          open ? 'nav-pill-active' : 'nav-pill'
        }`}
        title="Session and account"
      >
        <UserCircle className="h-4 w-4 shrink-0" aria-hidden />
        <span className="hidden sm:inline max-w-[5rem] truncate text-xs font-medium">{username}</span>
        <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div
          className={`absolute right-0 top-full z-50 mt-1.5 w-64 animate-scale-in rounded-xl border py-2 shadow-xl ${dropdownSurfaceClass()}`}
          role="menu"
        >
          <div className="px-3 py-2 glass-table-row">
            <div className="text-[10px] uppercase tracking-[0.14em] text-ink-3">{authModeLabel}</div>
            <div className="mt-0.5 truncate font-mono text-sm text-ink" title={bearerPreview}>
              {bearerPreview}
            </div>
            <div className="mt-2 flex items-center gap-2">
              <span className="text-sm text-ink-2">{username}</span>
              <span className="rounded-full border border-brand/30 bg-brand/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-brand">
                {role}
              </span>
            </div>
            {clusterCtx ? (
              <div className="mt-2 flex items-center gap-1.5 text-[11px] text-ink-2">
                <Container className="h-3 w-3 shrink-0 text-brand/80" />
                <span className="truncate" title={clusterCtx}>
                  {clusterCtx}
                </span>
              </div>
            ) : null}
          </div>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onLogout();
            }}
            className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${dropdownItemClass(false)}`}
          >
            <LogOut className="h-4 w-4 shrink-0" />
            Sign out
          </button>
        </div>
      )}
    </div>
  );
}

function NavPill({
  active,
  onClick,
  icon,
  label,
  compact,
}: {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  label: string;
  compact?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      className={`flex shrink-0 items-center gap-1.5 rounded-xl border text-sm font-medium transition-colors ${
        compact ? 'px-2 py-1.5' : 'px-3 py-2'
      } ${active ? 'nav-pill-active' : 'nav-pill'}`}
    >
      <span className="shrink-0">{icon}</span>
      <span className={compact ? 'hidden xl:inline' : 'whitespace-nowrap'}>{label}</span>
    </button>
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
  const { resolvedTheme, toggleDarkLight, accent, setAccent } = useTheme();
  const { role } = useAuth();
  const [proView, setProView] = useProView();
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

  const partitionedDropdownGroups = useMemo(
    () =>
      dropdownGroups.map((group) => {
        const { ready, setup } = partitionNavViews(group.items, platform, navExtras);
        return { ...group, ready, setup };
      }),
    [platform, navExtras],
  );

  const recentNavItems = useMemo(() => {
    return getRecentViews()
      .filter((view) => view !== currentView && view !== 'overview')
      .slice(0, 5)
      .map((view) => {
        try {
          return { view, label: getViewMeta(view).label };
        } catch {
          return null;
        }
      })
      .filter((item): item is { view: AppView; label: string } => item !== null);
  }, [currentView, mobileOpen]);

  const filteredMobileNavGroups = useMemo(() => {
    if (proView) {
      return [
        {
          label: 'AI OS',
          items: AI_OS_NAV.map((section) => ({
            label: section.label,
            view: section.primaryView as AppView,
            icon:
              section.id === 'overview' ? (
                <LayoutDashboard className="w-4 h-4" />
              ) : section.id === 'fabric' ? (
                <GitBranch className="w-4 h-4" />
              ) : (
                <Sparkles className="w-4 h-4" />
              ),
          })),
        },
        ...mobileNavGroups.slice(1),
      ].map((group) => ({
        ...group,
        items: filterNavViews(group.items, platform, navExtras),
      }));
    }
    const groups = [{ label: 'CloudOS', items: [{ label: 'Dashboard', view: 'overview' as AppView, icon: <LayoutDashboard className="w-4 h-4" /> }, ...cloudOsNavItems] }];
    return groups.map((group) => ({
      ...group,
      items: filterNavViews(group.items, platform, navExtras),
    }));
  }, [platform, navExtras, proView]);

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

  useEffect(() => {
    if (!mobileOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = prev;
    };
  }, [mobileOpen]);

  const desktopNav = proView ? (
    <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto pb-0.5">
      {AI_OS_NAV.map((section) => (
        <NavPill
          key={section.id}
          active={isAiOsNavActive(section, currentView)}
          onClick={() => onNavigate(section.primaryView)}
          icon={
            section.id === 'overview' ? (
              <LayoutDashboard className="w-4 h-4" />
            ) : section.id === 'fabric' ? (
              <GitBranch className="w-4 h-4" />
            ) : section.id === 'ai-studio' ? (
              <Brain className="w-4 h-4" />
            ) : section.id === 'cost' ? (
              <DollarSign className="w-4 h-4" />
            ) : section.id === 'security' ? (
              <Shield className="w-4 h-4" />
            ) : section.id === 'gitops' ? (
              <GitBranch className="w-4 h-4" />
            ) : section.id === 'labs' ? (
              <Sparkles className="w-4 h-4" />
            ) : section.id === 'settings' ? (
              <Settings className="w-4 h-4" />
            ) : section.id === 'fleet' ? (
              <Globe className="w-4 h-4" />
            ) : section.id === 'workloads' ? (
              <Container className="w-4 h-4" />
            ) : section.id === 'migrations' ? (
              <Rocket className="w-4 h-4" />
            ) : (
              <BarChart3 className="w-4 h-4" />
            )
          }
          label={section.label}
          compact
        />
      ))}
    </div>
  ) : (
    <>
      <NavPill
        active={currentView === 'overview'}
        onClick={() => onNavigate('overview')}
        icon={<LayoutDashboard className="w-4 h-4" />}
        label="Dashboard"
        compact
      />
      {partitionedDropdownGroups.map((group) => (
        <Dropdown
          key={group.label}
          group={group}
          ready={group.ready}
          setup={group.setup}
          isActive={group.items.some((item) => item.view === currentView)}
          currentView={currentView}
          onNavigate={onNavigate}
        />
      ))}
    </>
  );

  return (
    <nav className={`overflow-visible ${navbarShellClass()}`}>
      <div className="dash-content min-w-0">
        {/* Row 1: brand + utilities (always fits viewport). min-h-nav matches
            the 44px Apple-anatomy nav height from the Zyvor design reference. */}
        <div className="flex min-h-[var(--nav-h)] min-w-0 items-center justify-between gap-2 py-2 sm:py-2.5">
          <div className="flex min-w-0 items-center gap-2">
            <button
              type="button"
              onClick={() => onNavigate('overview')}
              className="group flex min-w-0 items-center gap-2 transition-opacity hover:opacity-90"
            >
              <div className="relative flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-2xl border border-brand/25 bg-gradient-to-br from-aether/15 to-aether-ai/10 shadow-[0_0_0_1px_rgba(59,130,246,0.08)] transition group-hover:border-brand/40 sm:h-10 sm:w-10">
                <div className="absolute inset-0 bg-gradient-to-br from-white/10 via-transparent to-aether/10" />
                <Hexagon className="h-5 w-5 text-brand" />
              </div>
              <div className="min-w-0 text-left">
                <span className="block truncate bg-gradient-to-r from-white to-slate-400 bg-clip-text text-base font-semibold tracking-tight text-transparent sm:text-lg">
                  Aether
                </span>
                <span className="hidden 2xl:block text-[10px] uppercase tracking-[0.2em] text-ink-3">
                  Universal Runtime Control Plane
                </span>
              </div>
            </button>
          </div>

          <div className="flex shrink-0 items-center gap-0.5 sm:gap-1">
            {!isMacOSShell() ? (
              <label className="hidden xl:flex items-center gap-1.5 shrink-0 text-xs text-ink-3" title="Classic CloudOS navigation (AI OS is default)">
                <input
                  type="checkbox"
                  checked={!proView}
                  onChange={(e) => setProView(!e.target.checked)}
                  className="accent-aether"
                  aria-label="Classic navigation"
                />
                Classic
              </label>
            ) : null}
            <button
              type="button"
              onClick={toggleDarkLight}
              aria-label={resolvedTheme === 'dark' ? 'Switch to light appearance' : 'Switch to dark appearance'}
              title={resolvedTheme === 'dark' ? 'Switch to light appearance' : 'Switch to dark appearance'}
              className="hidden lg:flex shrink-0 items-center justify-center rounded-xl border border-rule p-1.5 text-ink-2 transition-colors hover:bg-hover hover:text-ink"
            >
              {resolvedTheme === 'dark' ? <Sun className="h-3.5 w-3.5" aria-hidden /> : <Moon className="h-3.5 w-3.5" aria-hidden />}
            </button>
            <div className="hidden lg:flex shrink-0 items-center gap-1 rounded-xl border border-rule px-1.5 py-1" role="radiogroup" aria-label="Accent color">
              {ACCENT_OPTIONS.map((opt) => (
                <button
                  key={opt.value}
                  type="button"
                  role="radio"
                  aria-checked={accent === opt.value}
                  onClick={() => setAccent(opt.value)}
                  title={opt.label}
                  className={`h-3.5 w-3.5 shrink-0 rounded-full ${opt.swatchClass} transition-transform hover:scale-110 ${
                    accent === opt.value ? 'ring-2 ring-offset-1 ring-offset-page ring-ink-2' : ''
                  }`}
                >
                  <span className="sr-only">{opt.label}</span>
                </button>
              ))}
            </div>
            <PlatformHealthChip sseConnected={sseConnected ?? false} />
            {onOpenHelp ? (
              <div className="relative hidden sm:block shrink-0" ref={helpRef}>
                <button
                  type="button"
                  onClick={() => setHelpMenuOpen((v) => !v)}
                  aria-expanded={helpMenuOpen}
                  aria-haspopup="menu"
                  className={`flex items-center gap-1 rounded-xl border p-2 text-sm transition-colors ${
                    helpMenuOpen ? 'nav-pill-active' : 'nav-pill'
                  }`}
                  title="Help (?)"
                  aria-label="Help menu"
                >
                  <CircleHelp className="h-4 w-4 shrink-0" aria-hidden />
                  <span className="hidden xl:inline text-xs font-medium">Help</span>
                  <ChevronDown
                    className={`hidden xl:block h-3 w-3 transition-transform ${helpMenuOpen ? 'rotate-180' : ''}`}
                    aria-hidden
                  />
                </button>
                {helpMenuOpen && (
                  <div
                    className={`absolute right-0 top-full z-50 mt-1.5 min-w-[12.5rem] animate-scale-in rounded-xl border py-1.5 shadow-xl ${dropdownSurfaceClass()}`}
                    role="menu"
                  >
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setHelpMenuOpen(false);
                        onOpenHelp('shortcuts');
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${dropdownItemClass(false)}`}
                    >
                      <Keyboard className="h-4 w-4 shrink-0" aria-hidden />
                      Keyboard shortcuts
                      <kbd className="ml-auto rounded glass-inset-surface px-1 py-0.5 font-mono text-[10px] text-ink-3">?</kbd>
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      onClick={() => {
                        setHelpMenuOpen(false);
                        onOpenHelp('about');
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${dropdownItemClass(false)}`}
                    >
                      <Info className="h-4 w-4 shrink-0" aria-hidden />
                      About
                    </button>
                    <a
                      role="menuitem"
                      href={ZYVOR_HELP.docs}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => setHelpMenuOpen(false)}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${dropdownItemClass(false)}`}
                    >
                      <BookOpen className="h-4 w-4 shrink-0" aria-hidden />
                      Help &amp; documentation
                      <ExternalLink className="ml-auto h-3 w-3 opacity-60" aria-hidden />
                    </a>
                    <a
                      role="menuitem"
                      href={ZYVOR_HELP.contact}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => setHelpMenuOpen(false)}
                      className={`flex w-full items-center gap-2 px-3 py-2.5 text-sm transition-colors ${dropdownItemClass(false)}`}
                    >
                      <ExternalLink className="h-4 w-4 shrink-0" aria-hidden />
                      Contact support
                    </a>
                    <div className="glass-divider-t px-3 py-2 text-[11px]">
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
              data-testid="navbar-sse-status"
              className={`inline-block h-2 w-2 rounded-full sm:hidden ${sseConnected ? 'bg-emerald-400 platform-pulse' : 'bg-red-400'}`}
              title={sseConnected ? 'SSE Connected' : 'SSE Disconnected'}
            />
            <span className="hidden text-xs text-ink-3 md:inline">{relativeTime}</span>
            <button
              type="button"
              onClick={handleRefreshClick}
              className="nav-pill rounded-xl border p-2 transition-colors"
              title="Refresh"
            >
              <RefreshCw className={`h-4 w-4${spinning ? ' animate-spin' : ''}`} />
            </button>
            <TenantSwitcher />
            <AccountMenu
              username={username}
              role={role}
              authModeLabel={authModeLabel}
              bearerPreview={bearerPreview}
              clusterCtx={clusterCtx}
              onLogout={onLogout}
            />
            <button
              type="button"
              onClick={() => setMobileOpen((v) => !v)}
              className="nav-pill rounded-xl border p-2 transition-colors lg:hidden"
              aria-expanded={mobileOpen}
              aria-label={mobileOpen ? 'Close menu' : 'Open menu'}
            >
              {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </button>
          </div>
        </div>

        {/* Row 2: primary nav (wide screens only, scrolls horizontally if needed) */}
        <div className="hidden min-w-0 overflow-visible pb-2 lg:block">
          <div className="relative flex min-w-0 flex-wrap items-center gap-1">
            {desktopNav}
          </div>
        </div>
      </div>

      {/* Mobile / tablet drawer */}
      {mobileOpen && (
        <>
          <button
            type="button"
            className="glass-modal-backdrop fixed inset-0 z-50 lg:hidden"
            aria-label="Close menu"
            onClick={() => setMobileOpen(false)}
          />
          <aside
            className="copilot-rail-glass fixed inset-y-0 right-0 z-50 flex w-full max-w-sm animate-fade-in flex-col border-l glass-divider/40 shadow-2xl lg:hidden"
            aria-label="Navigation menu"
          >
            <div className="flex shrink-0 items-center justify-between glass-divider-b px-4 py-3">
              <span className="text-sm font-semibold text-ink">Menu</span>
              <button
                type="button"
                onClick={() => setMobileOpen(false)}
                className="rounded-xl p-2 text-ink-2 glass-nav-item hover:text-ink"
                aria-label="Close menu"
              >
                <X className="h-5 w-5" />
              </button>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
              <div className="space-y-4 px-4 py-3">
            {onOpenCommandPalette ? (
              <button
                type="button"
                data-testid="navbar-command-palette"
                onClick={() => {
                  onOpenCommandPalette();
                  setMobileOpen(false);
                }}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium border ${
                  'border-brand/30 bg-brand/10 text-brand hover:bg-brand/15'
                }`}
              >
                <Command className="w-4 h-4" />
                Command palette
                <kbd className="ml-auto text-[10px] opacity-70">⌘K</kbd>
              </button>
            ) : null}

            {recentNavItems.length > 0 ? (
              <div>
                <div className="pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-ink-3">
                  Recently visited
                </div>
                <div className="space-y-0.5">
                  {recentNavItems.map((item) => (
                    <button
                      key={item.view}
                      type="button"
                      onClick={() => handleMobileNavigate(item.view)}
                      className="w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium text-ink-2 transition-colors hover:text-ink glass-nav-item"
                    >
                      <Clock className="w-4 h-4 text-ink-3" />
                      {item.label}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}

            <div className="glass-panel-card rounded-2xl px-4 py-3">
              <div className="text-[10px] uppercase tracking-[0.2em] text-ink-3">{authModeLabel}</div>
              <div className="mt-1 text-sm font-medium font-mono tracking-tight text-ink">
                {bearerPreview}
              </div>
              <div className="mt-2 flex items-center gap-2">
                <span className="text-sm text-ink-2">{username}</span>
                <span className="rounded-full border border-brand/30 bg-brand/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-brand">
                  {role}
                </span>
              </div>
              {clusterCtx ? (
                <div className="mt-2 flex items-center gap-1.5 text-[11px] text-ink-2">
                  <Container className="h-3 w-3 shrink-0 text-brand/80" />
                  <span className="truncate" title={clusterCtx}>
                    {clusterCtx}
                  </span>
                </div>
              ) : null}
            </div>

            {filteredMobileNavGroups.map((group) => (
              group.items.length === 0 ? null : (
              <div key={group.label}>
                <div className="flex items-center gap-1.5 pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-ink-3">
                  <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${NAV_GROUP_COLOR[group.label.toLowerCase() as NavGroup] ?? 'bg-ink-3'}`} aria-hidden />
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
                          ? 'text-brand bg-brand/10'
                          : 'text-ink-2 hover:text-ink glass-nav-item'
                      }`}
                    >
                      <span className={currentView === item.view ? 'text-brand' : 'text-ink-3'}>
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
              <div>
                <div className="pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-ink-3">Help</div>
                <div className="space-y-0.5">
                  <button
                    type="button"
                    onClick={() => {
                      onOpenHelp('shortcuts');
                      setMobileOpen(false);
                    }}
                    className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-colors ${
                      'text-ink-2 hover:text-ink glass-nav-item'
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
                      'text-ink-2 hover:text-ink glass-nav-item'
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
                      'text-ink-2 hover:text-ink glass-nav-item'
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
                      'text-ink-2 hover:text-ink glass-nav-item'
                    }`}
                  >
                    <ExternalLink className="w-4 h-4" />
                    Contact support
                  </a>
                  <div
                    className={`px-4 py-2 text-[11px] ${
                      'text-ink-3'
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

            <div className="flex flex-wrap items-center gap-2 glass-divider-t/60 pt-2">
              <button
                type="button"
                onClick={toggleDarkLight}
                className="flex flex-1 min-w-[8rem] items-center gap-2 rounded-xl border border-rule px-2 py-2 text-xs text-ink-2 transition-colors hover:bg-hover hover:text-ink"
              >
                {resolvedTheme === 'dark' ? <Sun className="h-4 w-4 shrink-0" aria-hidden /> : <Moon className="h-4 w-4 shrink-0" aria-hidden />}
                {resolvedTheme === 'dark' ? 'Light appearance' : 'Dark appearance'}
              </button>
              <div className="flex w-full items-center justify-center gap-2 rounded-xl border border-rule px-2 py-2" role="radiogroup" aria-label="Accent color">
                {ACCENT_OPTIONS.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    role="radio"
                    aria-checked={accent === opt.value}
                    onClick={() => setAccent(opt.value)}
                    title={opt.label}
                    className={`h-4 w-4 shrink-0 rounded-full ${opt.swatchClass} ${accent === opt.value ? 'ring-2 ring-offset-1 ring-offset-page ring-ink-2' : ''}`}
                  >
                    <span className="sr-only">{opt.label}</span>
                  </button>
                ))}
              </div>
              <button
                type="button"
                onClick={() => {
                  onLogout();
                  setMobileOpen(false);
                }}
                className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm transition-colors ${
                  'text-ink-2 hover:text-ink glass-nav-item border glass-divider'
                }`}
              >
                <LogOut className="w-4 h-4" />
                Sign out
              </button>
            </div>
              </div>
            </div>
          </aside>
        </>
      )}
    </nav>
  );
}
