// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useMemo, useState, type ReactNode } from 'react';
import Navbar from './Navbar';
import Hero, { type HeroBadge } from './Hero';
import Footer from './Footer';
import CopilotRail, { CopilotRailToggle } from './CopilotRail';
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
  children: ReactNode;
  commandPalette: ReactNode;
  helpDialog: ReactNode;
  toastContainer: ReactNode;
  refreshKey?: number;
}

function buildHeroBadges(
  version: string | undefined,
  platform: ReturnType<typeof useServerCapabilities>['capabilities'],
  ready: ReturnType<typeof useServerCapabilities>['ready'],
  sseConnected: boolean,
): HeroBadge[] {
  if (!platform?.platform) {
    return version ? [{ label: `v${version}`, tone: 'brand' }] : [];
  }
  const p = platform.platform;
  const badges: HeroBadge[] = [{ label: `v${p.version}`, tone: 'brand' }];

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
  const [copilotCollapsed, setCopilotCollapsed] = useState(false);
  const [mobileCopilotOpen, setMobileCopilotOpen] = useState(false);
  const showCompactHero = currentView === 'overview' || currentView === 'fabric';

  return (
    <div className={shellClass}>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-xl focus:border focus:border-aether/30 focus:bg-[#11151C]/90 focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-aether focus:shadow-lg focus:backdrop-blur-xl"
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
      <ViewerBanner />
      {sseBannerVisible ? <SseReconnectBanner onRefresh={onRefresh} /> : null}
      <VersionRefreshBanner />
      {!showCompactHero ? (
        <Hero title={heroTitle} subtitle={heroSubtitle} badges={heroBadges} />
      ) : null}
      <div className="flex min-h-0 flex-1">
        <main id="main-content" className="min-w-0 flex-1 dash-content py-8 lg:py-10">{children}</main>
        <CopilotRail collapsed={copilotCollapsed} onCollapsedChange={setCopilotCollapsed} />
      </div>
      <CopilotRailToggle onClick={() => setMobileCopilotOpen(true)} />
      {mobileCopilotOpen ? (
        <div className="fixed inset-0 z-50 xl:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-[#11151C]/50 backdrop-blur-md"
            aria-label="Close copilot"
            onClick={() => setMobileCopilotOpen(false)}
          />
          <div className="absolute inset-y-0 right-0 flex w-full max-w-md">
            <CopilotRail collapsed={false} onCollapsedChange={() => setMobileCopilotOpen(false)} />
          </div>
        </div>
      ) : null}
      <Footer />
      <LiveActivityDock />
      <AgentStatusDock />
      <CriticalIssueNotifier refreshKey={refreshKey} />
      {commandPalette}
      {helpDialog}
      {toastContainer}
    </div>
  );
}
