import { useState, useRef, useCallback, useEffect, type ReactNode } from 'react';
import {
  Hexagon,
  LayoutDashboard,
  Container,
  Brain,
  DollarSign,
  Target,
  GitCompare,
  ShieldCheck,
  Settings,
  HeartPulse,
  Bell,
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
  LogOut,
  ChevronDown,
  Menu,
  X,
} from 'lucide-react';
import type { AppView } from '../types/api';

interface NavbarProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  username: string;
  onLogout: () => void;
  onRefresh: () => void;
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
  { label: 'Scheduler', view: 'scheduler', icon: <Settings className="w-4 h-4" /> },
  { label: 'Health Monitor', view: 'health', icon: <HeartPulse className="w-4 h-4" /> },
  { label: 'Events', view: 'events', icon: <Bell className="w-4 h-4" /> },
  { label: 'SLA Compliance', view: 'sla', icon: <FileCheck className="w-4 h-4" /> },
  { label: 'Dependencies', view: 'deps', icon: <GitBranch className="w-4 h-4" /> },
  { label: 'Environments', view: 'envs', icon: <Layers className="w-4 h-4" /> },
];

const resourcesItems: DropdownItem[] = [
  { label: 'Secrets', view: 'secrets', icon: <KeyRound className="w-4 h-4" /> },
  { label: 'Backups', view: 'backups', icon: <Archive className="w-4 h-4" /> },
  { label: 'Templates', view: 'templates', icon: <FileCode2 className="w-4 h-4" /> },
  { label: 'Plugins', view: 'plugins', icon: <Puzzle className="w-4 h-4" /> },
  { label: 'Audit Trail', view: 'audit', icon: <ClipboardList className="w-4 h-4" /> },
  { label: 'Metrics', view: 'metrics', icon: <BarChart3 className="w-4 h-4" /> },
];

const dropdownGroups: DropdownGroup[] = [
  { label: 'Intelligence', items: intelligenceItems },
  { label: 'Operations', items: operationsItems },
  { label: 'Resources', items: resourcesItems },
];

function Dropdown({
  group,
  isActive,
  currentView,
  onNavigate,
}: {
  group: DropdownGroup;
  isActive: boolean;
  currentView: AppView;
  onNavigate: (view: AppView) => void;
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
        className={`flex items-center gap-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
          isActive
            ? 'text-aether bg-aether/10'
            : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800'
        }`}
      >
        {group.label}
        <ChevronDown className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div className="absolute top-full left-0 mt-1 w-56 bg-zinc-900 border border-zinc-700 rounded-xl shadow-xl py-2 z-50 animate-scale-in">
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
                  : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800'
              }`}
            >
              <span className={currentView === item.view ? 'text-aether' : 'text-zinc-500'}>
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

export default function Navbar({ currentView, onNavigate, username, onLogout, onRefresh, lastRefreshed, sseConnected }: NavbarProps) {
  const [mobileOpen, setMobileOpen] = useState(false);
  const [spinning, setSpinning] = useState(false);
  const relativeTime = useRelativeTime(lastRefreshed);

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

  // All nav items flattened for mobile menu
  const allNavItems: DropdownItem[] = [
    { label: 'Dashboard', view: 'overview', icon: <LayoutDashboard className="w-4 h-4" /> },
    { label: 'Workloads', view: 'workloads', icon: <Container className="w-4 h-4" /> },
    ...intelligenceItems,
    ...operationsItems,
    ...resourcesItems,
  ];

  return (
    <nav className="sticky top-0 z-40 navbar-blur border-b border-zinc-800/60">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Left: Logo */}
          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={() => onNavigate('overview')}
              className="flex items-center gap-2 hover:opacity-80 transition-opacity"
            >
              <div className="w-8 h-8 rounded-lg bg-aether/10 flex items-center justify-center">
                <Hexagon className="w-5 h-5 text-aether" />
              </div>
              <span className="text-lg font-bold text-white tracking-tight">Aether</span>
            </button>
          </div>

          {/* Center: Navigation (desktop) */}
          <div className="hidden md:flex items-center gap-1">
            <button
              onClick={() => onNavigate('overview')}
              className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                currentView === 'overview'
                  ? 'text-aether bg-aether/10'
                  : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800'
              }`}
            >
              <LayoutDashboard className="w-4 h-4" />
              Dashboard
            </button>

            <button
              onClick={() => onNavigate('workloads')}
              className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                currentView === 'workloads'
                  ? 'text-aether bg-aether/10'
                  : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800'
              }`}
            >
              <Container className="w-4 h-4" />
              Workloads
            </button>

            {dropdownGroups.map((group) => {
              const isActive = group.items.some((item) => item.view === currentView);
              return (
                <Dropdown
                  key={group.label}
                  group={group}
                  isActive={isActive}
                  currentView={currentView}
                  onNavigate={onNavigate}
                />
              );
            })}
          </div>

          {/* Right: Actions */}
          <div className="flex items-center gap-2">
            <span
              className={`hidden sm:inline-block w-2 h-2 rounded-full ${sseConnected ? 'bg-emerald-400 animate-pulse' : 'bg-red-400'}`}
              title={sseConnected ? 'SSE Connected' : 'SSE Disconnected'}
            />
            <span className="hidden sm:inline text-xs text-zinc-500">{relativeTime}</span>
            <button
              onClick={handleRefreshClick}
              className="p-2 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors"
              title="Refresh"
            >
              <RefreshCw className={`w-4 h-4${spinning ? ' animate-spin' : ''}`} />
            </button>

            <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-lg bg-zinc-800 border border-zinc-700">
              <div className="w-2 h-2 rounded-full bg-emerald-400" />
              <span className="text-sm text-zinc-300">{username}</span>
            </div>

            <button
              onClick={onLogout}
              className="p-2 rounded-lg text-zinc-400 hover:text-red-400 hover:bg-red-500/10 transition-colors"
              title="Logout"
            >
              <LogOut className="w-4 h-4" />
            </button>

            {/* Mobile hamburger */}
            <button
              onClick={() => setMobileOpen((v) => !v)}
              className="md:hidden p-2 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 transition-colors"
              aria-label="Toggle menu"
            >
              {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
            </button>
          </div>
        </div>
      </div>

      {/* Mobile menu overlay */}
      {mobileOpen && (
        <div className="md:hidden border-t border-zinc-800/60 bg-zinc-950/95 backdrop-blur-lg animate-fade-in">
          <div className="max-w-7xl mx-auto px-4 py-3 space-y-1">
            {allNavItems.map((item) => (
              <button
                key={item.view}
                onClick={() => handleMobileNavigate(item.view)}
                className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                  currentView === item.view
                    ? 'text-aether bg-aether/10'
                    : 'text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800'
                }`}
              >
                <span className={currentView === item.view ? 'text-aether' : 'text-zinc-500'}>
                  {item.icon}
                </span>
                {item.label}
              </button>
            ))}
          </div>
        </div>
      )}
    </nav>
  );
}
