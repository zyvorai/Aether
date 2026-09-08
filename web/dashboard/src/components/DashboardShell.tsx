// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, type ReactNode } from 'react';
import GlobalNav from './layout/GlobalNav/GlobalNav';
import AetherSidebar from './layout/AetherSidebar/AetherSidebar';
import Breadcrumb from './Breadcrumb';
import ZyraRail from './ZyraRail';
import CriticalIssueNotifier from './CriticalIssueNotifier';
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
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);

  useEffect(() => {
    setMobileSidebarOpen(false);
  }, [currentView]);

  return (
    <div className={shellClass}>
      <div className="ae-ambient" aria-hidden><i /><i /><i /></div>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-xl focus:border focus:border-primary/30 focus:glass border border-border shadow-card focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-primary focus:shadow-lg"
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
        onOpenMobileSidebar={() => setMobileSidebarOpen(true)}
        sseConnected={sseConnected}
      />
      <div className="relative z-[1]">
        <ViewerBanner />
        {sseBannerVisible ? <SseReconnectBanner onRefresh={onRefresh} /> : null}
        <VersionRefreshBanner />
      </div>
      <div className="relative z-[1] flex min-h-0 flex-1">
        <AetherSidebar currentView={currentView} onNavigate={onNavigate} />
        <main id="main-content" className="min-w-0 flex-1 dash-content py-5 lg:py-6">
          <Breadcrumb currentView={currentView} onNavigate={onNavigate} workloadName={breadcrumbWorkload} />
          {children}
        </main>
        {isZyraView ? (
          <ZyraRail collapsed={false} onCollapsedChange={() => onNavigate('overview')} />
        ) : null}
      </div>
      {mobileSidebarOpen ? (
        <div className="fixed inset-0 z-50 lg:hidden">
          <button
            type="button"
            className="glass-modal-backdrop absolute inset-0"
            aria-label="Close navigation"
            onClick={() => setMobileSidebarOpen(false)}
          />
          <div className="absolute inset-y-0 left-0 flex">
            <AetherSidebar
              currentView={currentView}
              onNavigate={onNavigate}
              mobile
              onNavigateMobile={() => setMobileSidebarOpen(false)}
            />
          </div>
        </div>
      ) : null}
      <CriticalIssueNotifier refreshKey={refreshKey} />
      {commandPalette}
      {helpDialog}
      {toastContainer}
    </div>
  );
}
