// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { ChevronDown, CircleHelp, Info, Keyboard, LogOut, Menu, Palette, Search, Settings } from 'lucide-react';
import { useNavigate } from 'react-router';
import ThemeToggle from '../../ThemeToggle';
import { AetherLockup } from '../../brand';
import type { HelpTab } from '../../HelpDialog';
import type { AppView } from '../../../types/api';
import { apiFetchHealth, type HealthPayload } from '../../../utils/api';
import { isMacPlatform } from '../../../utils/platformKeys';
import NavMenu from './NavMenu';
import LiveStatus, { type LiveStatusProps } from './LiveStatus';
import EventInbox from './EventInbox';
import styles from './GlobalNav.module.css';

export interface GlobalNavProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  username: string;
  onLogout: () => void;
  onSearchClick?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  onOpenMobileSidebar?: () => void;
  live?: LiveStatusProps;
  refreshKey?: number;
}

export function GlobalNav({
  onNavigate, username, onLogout, onSearchClick, onOpenHelp, onOpenMobileSidebar, live, refreshKey,
}: GlobalNavProps) {
  const navigate = useNavigate();
  const [health, setHealth] = useState<HealthPayload | null>(null);

  useEffect(() => {
    let cancelled = false;
    void apiFetchHealth().then((payload) => {
      if (!cancelled) setHealth(payload);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <nav className={styles.gnav} aria-label="Global" data-testid="global-nav">
      <div className={styles.gnavInner}>
        <button type="button" className={styles.burger} aria-label="Open navigation" onClick={() => onOpenMobileSidebar?.()}>
          <Menu size={20} aria-hidden />
        </button>
        <button className={styles.mark} type="button" onClick={() => navigate('/')}>
          <AetherLockup size={18} wordmarkClassName={styles.word} />
          <span className="sr-only">aether home</span>
        </button>
        {health ? (
          <span
            className={`hidden sm:inline-flex ${styles.envBadge}`}
            title={`Host: ${health.hostname}${health.environment ? ` · Environment: ${health.environment}` : ''}`}
          >
            {health.environment ?? health.hostname}
          </span>
        ) : null}

        {onSearchClick ? (
          <button type="button" className={styles.searchBtn} onClick={onSearchClick} aria-label="Search pages, workloads and actions">
            <Search size={15} aria-hidden />
            <span className={styles.searchText}>Search pages, workloads, actions</span>
            <kbd aria-hidden="true" className={styles.searchKbd}>{isMacPlatform ? '⌘K' : 'Ctrl K'}</kbd>
          </button>
        ) : null}

        <div className={styles.utils}>
          {live ? <LiveStatus {...live} /> : null}
          <EventInbox refreshKey={refreshKey} />
          {onOpenHelp ? (
            <NavMenu label="Help menu" title="Help" trigger={<CircleHelp size={17} aria-hidden />}>
              {(close) => (
                <>
                  <button type="button" role="menuitem" className={styles.menuItem} onClick={() => { close(); onOpenHelp('shortcuts'); }}>
                    <Keyboard size={14} aria-hidden />Keyboard shortcuts
                  </button>
                  <button type="button" role="menuitem" className={styles.menuItem} onClick={() => { close(); onOpenHelp('about'); }}>
                    <Info size={14} aria-hidden />About Aether
                  </button>
                </>
              )}
            </NavMenu>
          ) : null}
          <ThemeToggle />
          <NavMenu
            label={`Account menu for ${username}`}
            triggerClassName={styles.accountBtn}
            trigger={
              <>
                <span className={styles.accountAvatar} aria-hidden>{username.trim().charAt(0) || 'A'}</span>
                <ChevronDown size={14} className={styles.accountChevron} aria-hidden />
              </>
            }
          >
            {(close) => (
              <>
                <p className={styles.menuCaption}>Signed in as {username}</p>
                <button type="button" role="menuitem" className={styles.menuItem} onClick={() => { close(); onNavigate('settings'); }}>
                  <Settings size={14} aria-hidden />Settings
                </button>
                <button type="button" role="menuitem" className={styles.menuItem} onClick={() => { close(); onNavigate('settings-nav'); }}>
                  <Palette size={14} aria-hidden />Appearance and navigation
                </button>
                <div role="separator" className={styles.menuDivider} />
                <button type="button" role="menuitem" className={styles.menuDanger} onClick={onLogout}>
                  <LogOut size={14} aria-hidden />Sign out
                </button>
              </>
            )}
          </NavMenu>
        </div>
      </div>
    </nav>
  );
}

export default GlobalNav;
