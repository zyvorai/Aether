// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useMemo, useState, type ReactNode } from 'react';
import Navbar from './Navbar';
import Breadcrumb from './Breadcrumb';
import PageHeader, { type HeaderPill } from './PageHeader';
import ZyraRail, { ZyraRailToggle } from './ZyraRail';
import ZyraContextBar from './ZyraContextBar';
import AgentStatusDock from './AgentStatusDock';
import CriticalIssueNotifier from './CriticalIssueNotifier';
import LiveActivityDock from './LiveActivityDock';
import SseReconnectBanner from './SseReconnectBanner';
import VersionRefreshBanner from './VersionRefreshBanner';
import ViewerBanner from './ViewerBanner';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';
import type { HelpTab } from './HelpDialog';
import type { AppView } from '../types/api';

interface DashboardShellProps {
  shellClass: string;
  currentView: AppView;
  heroTitle: string;
  heroSubtitle: string;
  username: string;
  lastRefreshed: Date;
  sseConnected: boolean;
  sseBannerVisible?: boolean;
  onNavigate: (view: AppView) => void;
  onLogout: () => void;
  onRefresh: () => void;
  onOpenCommandPalette?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  breadcrumbWorkload?: string;
  children: ReactNode;
  commandPalette: ReactNode;
  helpDialog: ReactNode;
  toastContainer: ReactNode;
  refreshKey?: number;
}

// Views that render their own in-page masthead / hero and should NOT get the
// shared compact PageHeader (avoids a double title bar).
const SELF_MASTHEAD_VIEWS: ReadonlySet<AppView> = new Set<AppView>([
  'overview',
  'fabric',
  'workloads',
  'zyra',
  'copilot',
]);

function buildHeroBadges(
  version: string | undefined,
  platform: ReturnType<typeof useServerCapabilities>['capabilities'],
  ready: ReturnType<typeof useServerCapabilities>['ready'],
  sseConnected: boolean,
): HeaderPill[] {
  if (!platform?.platform) {
    return version ? [{ label: `v${version}`, tone: 'brand' }] : [];
  }
  const p = platform.platform;
  const badges: HeaderPill[] = [{ label: `v${p.version}`, tone: 'brand' }];

  if (p.workloadState.backend === 'postgresql') {
    badges.push({
      label: ready?.checks?.postgres?.ok === false ? 'Postgres offline' : 'PostgreSQL HA',
      tone: ready?.checks?.postgres?.ok === false ? 'warn' : 'ok',
    });
  } else {
    badges.push({ label: 'Local state', tone: 'info' });
  }

  if (p.haSharedCache) {
    badges.push({
      label: ready?.checks?.redis?.ok === false ? 'Redis offline' : 'Redis sessions',
      tone: ready?.checks?.redis?.ok === false ? 'warn' : 'info',
    });
  }

  if (p.tls) {
    badges.push({ label: 'TLS', tone: 'ok' });
  }

  if (p.oidc.enabled) {
    badges.push({ label: 'OIDC', tone: 'ok' });
  }

  badges.push({
    label: sseConnected ? 'Live updates' : 'Reconnecting…',
    tone: sseConnected ? 'ok' : 'warn',
  });

  return badges;
}

export default function DashboardShell({
  shellClass,
  currentView,
  heroTitle,
  heroSubtitle,
  username,
  lastRefreshed,
  sseConnected,
  sseBannerVisible = false,
  onNavigate,
  onLogout,
  onRefresh,
  onOpenCommandPalette,
  onOpenHelp,
  breadcrumbWorkload,
  children,
  commandPalette,
  helpDialog,
  toastContainer,
  refreshKey = 0,
}: DashboardShellProps) {
  const { capabilities, ready } = useServerCapabilities();
  const heroBadges = useMemo(
    () => buildHeroBadges(capabilities?.version, capabilities, ready, sseConnected),
    [capabilities, ready, sseConnected],
  );
  const isZyraView = currentView === 'zyra' || currentView === 'copilot';
  const [zyraCollapsed, setZyraCollapsed] = useState(() => !isZyraView);
  const [mobileZyraOpen, setMobileZyraOpen] = useState(false);
  const showSharedHeader = !SELF_MASTHEAD_VIEWS.has(currentView);

  // Reclaim horizontal space on every view except the Zyra workspace, where the
  // rail IS the primary surface.
  useEffect(() => {
    setZyraCollapsed(!isZyraView);
  }, [currentView, isZyraView]);

  return (
    <div className={shellClass}>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-xl focus:border focus:border-brand/30 focus:glass-dropdown-surface focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-brand focus:shadow-lg"
      >
        Skip to content
      </a>
      <Navbar
        currentView={currentView}
        onNavigate={onNavigate}
        username={username}
        onLogout={onLogout}
        onRefresh={onRefresh}
        onOpenCommandPalette={onOpenCommandPalette}
        onOpenHelp={onOpenHelp}
        lastRefreshed={lastRefreshed}
        sseConnected={sseConnected}
      />
      <ZyraContextBar refreshKey={refreshKey} />
      <ViewerBanner />
      {sseBannerVisible ? <SseReconnectBanner onRefresh={onRefresh} /> : null}
      <VersionRefreshBanner />
      <div className="flex min-h-0 flex-1">
        <main id="main-content" className="min-w-0 flex-1 dash-content py-5 lg:py-6">
          <Breadcrumb currentView={currentView} onNavigate={onNavigate} workloadName={breadcrumbWorkload} />
          {showSharedHeader ? (
            <PageHeader
              title={heroTitle}
              subtitle={heroSubtitle}
              eyebrow="Control Plane"
              pills={heroBadges}
              testId="page-header"
            />
          ) : null}
          {children}
        </main>
        <ZyraRail collapsed={zyraCollapsed} onCollapsedChange={setZyraCollapsed} />
      </div>
      <ZyraRailToggle onClick={() => setMobileZyraOpen(true)} />
      {mobileZyraOpen ? (
        <div className="fixed inset-0 z-50 xl:hidden">
          <button
            type="button"
            className="glass-modal-backdrop absolute inset-0"
            aria-label="Close Zyra"
            onClick={() => setMobileZyraOpen(false)}
          />
          <div className="absolute inset-y-0 right-0 flex w-full max-w-md">
            <ZyraRail collapsed={false} onCollapsedChange={() => setMobileZyraOpen(false)} />
          </div>
        </div>
      ) : null}
      <LiveActivityDock />
      <AgentStatusDock />
      <CriticalIssueNotifier refreshKey={refreshKey} />
      {commandPalette}
      {helpDialog}
      {toastContainer}
    </div>
  );
}
