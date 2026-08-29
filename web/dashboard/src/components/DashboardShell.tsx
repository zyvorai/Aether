// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, type ReactNode } from 'react';
import GlobalNav from './layout/GlobalNav/GlobalNav';
import Breadcrumb from './Breadcrumb';
import ZyraRail, { ZyraRailToggle } from './ZyraRail';
import ZyraContextBar from './ZyraContextBar';
import AgentStatusDock from './AgentStatusDock';
import CriticalIssueNotifier from './CriticalIssueNotifier';
import LiveActivityDock from './LiveActivityDock';
import SseReconnectBanner from './SseReconnectBanner';
import VersionRefreshBanner from './VersionRefreshBanner';
import ViewerBanner from './ViewerBanner';
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

export default function DashboardShell({
  shellClass,
  currentView,
  username,
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
  const isZyraView = currentView === 'zyra' || currentView === 'copilot';
  const [zyraCollapsed, setZyraCollapsed] = useState(() => !isZyraView);
  const [mobileZyraOpen, setMobileZyraOpen] = useState(false);

  // Reclaim horizontal space on every view except the Zyra workspace, where the
  // rail IS the primary surface.
  useEffect(() => {
    setZyraCollapsed(!isZyraView);
  }, [currentView, isZyraView]);

  return (
    <div className={shellClass}>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-xl focus:border focus:border-brand/30 focus:glass border border-border shadow-card focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-brand focus:shadow-lg"
      >
        Skip to content
      </a>
      <GlobalNav
        currentView={currentView}
        onNavigate={onNavigate}
        username={username}
        onLogout={onLogout}
        onSearchClick={onOpenCommandPalette}
        onOpenHelp={onOpenHelp}
        sseConnected={sseConnected}
      />
      <ZyraContextBar refreshKey={refreshKey} />
      <ViewerBanner />
      {sseBannerVisible ? <SseReconnectBanner onRefresh={onRefresh} /> : null}
      <VersionRefreshBanner />
      <div className="flex min-h-0 flex-1">
        <main id="main-content" className="min-w-0 flex-1 dash-content py-5 lg:py-6">
          <Breadcrumb currentView={currentView} onNavigate={onNavigate} workloadName={breadcrumbWorkload} />
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
