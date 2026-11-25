// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { ChevronDown, CircleHelp, LogOut, Menu, Search, Settings } from 'lucide-react';
import { useNavigate } from 'react-router';
import ThemeToggle from '../../ThemeToggle';
import type { HelpTab } from '../../HelpDialog';
import type { AppView } from '../../../types/api';
import { apiFetchHealth, type HealthPayload } from '../../../utils/api';
import styles from './GlobalNav.module.css';

export interface GlobalNavProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  username: string;
  onLogout: () => void;
  onSearchClick?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  onOpenMobileSidebar?: () => void;
  subnav?: React.ReactNode;
  sseConnected?: boolean;
}

export function GlobalNav({
  onNavigate, username, onLogout, onSearchClick, onOpenHelp, onOpenMobileSidebar, subnav, sseConnected,
}: GlobalNavProps) {
  const navigate = useNavigate();
  const [accountOpen, setAccountOpen] = useState(false);
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

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setAccountOpen(false);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, []);

  return (
    <nav className={styles.gnav} aria-label="Global" data-open={String(accountOpen)} data-testid="global-nav">
      <div className={styles.gnavInner}>
        <button type="button" className={styles.burger} aria-label="Open navigation" onClick={() => onOpenMobileSidebar?.()}>
          <Menu size={20} aria-hidden />
        </button>
        <button className={styles.mark} type="button" onClick={() => navigate('/')}>
          <span className={styles.markBadge} aria-hidden="true">Æ</span>
          <span className={styles.word}>aether</span>
          <span className="sr-only"> home</span>
        </button>
        {health ? (
          <span
            className={`hidden sm:inline-flex ${styles.envBadge}`}
            title={`Host: ${health.hostname}${health.environment ? ` · Environment: ${health.environment}` : ''}`}
          >
            {health.environment ?? health.hostname}
          </span>
        ) : null}

        <div className={styles.utils}>
          {sseConnected !== undefined ? <span className={styles.statusDot} data-connected={String(sseConnected)} role="img" aria-label={sseConnected ? 'Live updates connected' : 'Reconnecting live updates'} title={sseConnected ? 'Live updates connected' : 'Reconnecting live updates'} /> : null}
          {onSearchClick ? <button type="button" className={styles.searchBtn} onClick={onSearchClick}><Search size={16} aria-hidden /><span className="sr-only sm:not-sr-only">Search</span><kbd aria-hidden="true" className={`hidden sm:inline ${styles.searchKbd}`}>⌘K</kbd></button> : null}
          {onOpenHelp ? <button type="button" className={styles.icon} aria-label="Help menu" title="Help" onClick={() => onOpenHelp()}><CircleHelp size={17} aria-hidden /></button> : null}
          <ThemeToggle />
          <div className={styles.accountWrap}>
            <button type="button" className={styles.accountBtn} onClick={() => setAccountOpen((open) => !open)} aria-haspopup="menu" aria-expanded={accountOpen} aria-label={`Account menu for ${username}`}>
              <span className={styles.accountAvatar}>{username.trim().charAt(0) || 'A'}</span><ChevronDown size={14} />
            </button>
            {accountOpen ? (
              <>
                <button type="button" className="fixed inset-0 z-[58] cursor-default" aria-label="Close account menu" onClick={() => setAccountOpen(false)} />
                <div role="menu" className={styles.accountMenu}>
                  <button type="button" role="menuitem" className={styles.accountMenuItem} onClick={() => { setAccountOpen(false); onNavigate('settings'); }}><span className="inline-flex items-center gap-2"><Settings size={14} />Settings</span></button>
                  <div role="separator" className={styles.accountMenuDivider} />
                  <button type="button" role="menuitem" className={styles.accountMenuDanger} onClick={onLogout}><span className="inline-flex items-center gap-2"><LogOut size={14} />Sign out</span></button>
                </div>
              </>
            ) : null}
          </div>
        </div>
      </div>
      {subnav ? <div className={styles.subnav}>{subnav}</div> : null}
    </nav>
  );
}

export default GlobalNav;
