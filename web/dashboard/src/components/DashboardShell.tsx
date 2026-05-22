import { useMemo, type ReactNode } from 'react';
import Navbar from './Navbar';
import Hero, { type HeroBadge } from './Hero';
import Footer from './Footer';
import VersionRefreshBanner from './VersionRefreshBanner';
import PlatformBanner from './PlatformBanner';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';
import type { AppView } from '../types/api';

interface DashboardShellProps {
  shellClass: string;
  currentView: AppView;
  heroTitle: string;
  heroSubtitle: string;
  username: string;
  lastRefreshed: Date;
  sseConnected: boolean;
  onNavigate: (view: AppView) => void;
  onLogout: () => void;
  onRefresh: () => void;
  children: ReactNode;
  commandPalette: ReactNode;
  shortcutsHelp: ReactNode;
  toastContainer: ReactNode;
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
  onNavigate,
  onLogout,
  onRefresh,
  children,
  commandPalette,
  shortcutsHelp,
  toastContainer,
}: DashboardShellProps) {
  const { capabilities, ready } = useServerCapabilities();
  const heroBadges = useMemo(
    () => buildHeroBadges(capabilities?.version, capabilities, ready, sseConnected),
    [capabilities, ready, sseConnected],
  );

  return (
    <div className={shellClass}>
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[100] focus:rounded-lg focus:bg-aether focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-white"
      >
        Skip to content
      </a>
      <Navbar
        currentView={currentView}
        onNavigate={onNavigate}
        username={username}
        onLogout={onLogout}
        onRefresh={onRefresh}
        lastRefreshed={lastRefreshed}
        sseConnected={sseConnected}
      />
      <VersionRefreshBanner />
      <div className="dash-content pt-0">
        <PlatformBanner />
      </div>
      <Hero title={heroTitle} subtitle={heroSubtitle} badges={heroBadges} />
      <main id="main-content" className="flex-1 dash-content py-8 lg:py-10">{children}</main>
      <Footer />
      {commandPalette}
      {shortcutsHelp}
      {toastContainer}
    </div>
  );
}
