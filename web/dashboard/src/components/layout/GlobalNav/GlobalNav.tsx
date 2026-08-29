import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { ChevronDown, CircleHelp, LogOut, Menu, Moon, Search, Settings, Sun, X } from 'lucide-react';
import { useNavigate } from 'react-router';
import { useTheme } from '../../../contexts/ThemeContext';
import type { HelpTab } from '../../HelpDialog';
import type { AppView } from '../../../types/api';
import { NAV_DIRECT_LINKS, NAV_FLYOUT_PANELS } from '../../../utils/navFlyout';
import styles from './GlobalNav.module.css';

const CLOSE_DELAY_MS = 280;

export interface GlobalNavProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  username: string;
  onLogout: () => void;
  onSearchClick?: () => void;
  onOpenHelp?: (tab?: HelpTab) => void;
  subnav?: ReactNode;
  sseConnected?: boolean;
}

function ChevronIcon() {
  return <svg width="9" height="14" viewBox="0 0 9 14" fill="none" aria-hidden="true"><path d="M1.5 1L7 7l-5.5 6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" /></svg>;
}

export function GlobalNav({
  currentView, onNavigate, username, onLogout, onSearchClick, onOpenHelp, subnav, sseConnected,
}: GlobalNavProps) {
  const navigate = useNavigate();
  const { resolvedTheme, toggleDarkLight } = useTheme();
  const [openKey, setOpenKey] = useState<string | null>(null);
  const [sheetOpen, setSheetOpen] = useState(false);
  const [sheetSection, setSheetSection] = useState<string | null>(null);
  const [accountOpen, setAccountOpen] = useState(false);
  const closeTimer = useRef<ReturnType<typeof setTimeout>>();
  const triggerRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const navOpen = Boolean(openKey) || sheetOpen || accountOpen;

  const clearCloseTimer = useCallback(() => {
    if (closeTimer.current) clearTimeout(closeTimer.current);
  }, []);
  const scheduleClose = useCallback(() => {
    clearCloseTimer();
    closeTimer.current = setTimeout(() => setOpenKey(null), CLOSE_DELAY_MS);
  }, [clearCloseTimer]);
  const go = useCallback((view: AppView) => {
    setOpenKey(null);
    setSheetOpen(false);
    setSheetSection(null);
    onNavigate(view);
  }, [onNavigate]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      if (openKey) triggerRefs.current[openKey]?.focus();
      setOpenKey(null);
      setSheetOpen(false);
      setAccountOpen(false);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [openKey]);
  useEffect(() => {
    document.body.style.overflow = sheetOpen ? 'hidden' : '';
    return () => { document.body.style.overflow = ''; };
  }, [sheetOpen]);
  useEffect(() => () => clearCloseTimer(), [clearCloseTimer]);

  return (
    <>
      <nav
        className={styles.gnav}
        aria-label="Global"
        data-open={String(navOpen)}
        data-testid="global-nav"
        onMouseLeave={scheduleClose}
      >
        <div className={styles.gnavInner}>
          <button className={styles.mark} type="button" aria-label="Aether home" onClick={() => navigate('/')}>
            <span className={styles.markBadge}>Æ</span>
            <span className={styles.word}>aether</span>
          </button>

          <div className={styles.links}>
            {NAV_DIRECT_LINKS.map((item) => (
              <button key={item.view} type="button" className={styles.link} aria-current={currentView === item.view ? 'page' : undefined} onClick={() => go(item.view)}>
                {item.view === 'fabric' ? 'Fabric' : item.label}
              </button>
            ))}
            {NAV_FLYOUT_PANELS.map((panel) => (
              <button
                key={panel.key}
                ref={(element) => { triggerRefs.current[panel.key] = element; }}
                type="button"
                className={styles.link}
                aria-expanded={openKey === panel.key}
                aria-controls="aether-nav-flyout"
                onClick={() => setOpenKey((current) => current === panel.key ? null : panel.key)}
                onMouseEnter={() => { clearCloseTimer(); setOpenKey(panel.key); }}
              >
                {panel.label}
              </button>
            ))}
          </div>

          <div className={styles.utils}>
            {sseConnected !== undefined ? <span className={styles.statusDot} data-connected={String(sseConnected)} title={sseConnected ? 'Live updates connected' : 'Reconnecting live updates'} /> : null}
            {onSearchClick ? <button type="button" className={styles.searchBtn} onClick={onSearchClick}><Search size={15} /><span className="hidden sm:inline">Search</span><kbd className={`hidden sm:inline ${styles.searchKbd}`}>⌘K</kbd></button> : null}
            {onOpenHelp ? <button type="button" className={styles.icon} aria-label="Help" title="Help" onClick={() => onOpenHelp()}><CircleHelp size={16} /></button> : null}
            <button type="button" className={styles.icon} onClick={toggleDarkLight} aria-label={`Switch to ${resolvedTheme === 'dark' ? 'light' : 'dark'} theme`}>
              {resolvedTheme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
            </button>
            <div className={styles.accountWrap}>
              <button type="button" className={styles.accountBtn} onClick={() => setAccountOpen((open) => !open)} aria-haspopup="menu" aria-expanded={accountOpen}>
                <span className={styles.accountAvatar}>{username.trim().charAt(0) || 'A'}</span><ChevronDown size={14} />
              </button>
              {accountOpen ? (
                <>
                  <button type="button" className="fixed inset-0 z-[58] cursor-default" aria-label="Close account menu" onClick={() => setAccountOpen(false)} />
                  <div role="menu" className={styles.accountMenu}>
                    <button type="button" className={styles.accountMenuItem} onClick={() => { setAccountOpen(false); go('settings'); }}><span className="inline-flex items-center gap-2"><Settings size={14} />Settings</span></button>
                    <div className={styles.accountMenuDivider} />
                    <button type="button" className={styles.accountMenuDanger} onClick={onLogout}><span className="inline-flex items-center gap-2"><LogOut size={14} />Sign out</span></button>
                  </div>
                </>
              ) : null}
            </div>
            <button type="button" className={styles.burger} aria-expanded={sheetOpen} aria-controls="aether-nav-sheet" aria-label={sheetOpen ? 'Close menu' : 'Menu'} onClick={() => setSheetOpen((open) => !open)}>
              {sheetOpen ? <X size={17} /> : <Menu size={17} />}
            </button>
          </div>
        </div>
        {subnav ? <div className={styles.subnav}>{subnav}</div> : null}
      </nav>

      <div className={styles.fly} id="aether-nav-flyout" data-open={String(Boolean(openKey))} onMouseEnter={clearCloseTimer} onMouseLeave={scheduleClose}>
        <div className={styles.flyInner}>
          {NAV_FLYOUT_PANELS.map((panel) => (
            <div key={panel.key} className={styles.flyPanel} data-active={openKey === panel.key}>
              <div className={styles.flyCols}>
                {panel.groups.map((group) => (
                  <div key={group.heading} className={`${styles.flyGroup} ${group.lead ? styles.flyLead : ''}`}>
                    <h3>{group.heading}</h3>
                    <ul>{group.links.map((item) => <li key={item.view}><button type="button" onClick={() => go(item.view)}>{item.label}<small>{item.subtitle}</small></button></li>)}</ul>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
      <button type="button" aria-label="Close navigation" className={styles.flyScrim} data-open={String(Boolean(openKey))} onClick={() => setOpenKey(null)} />

      <div className={styles.sheet} id="aether-nav-sheet" data-open={String(sheetOpen)}>
        {NAV_DIRECT_LINKS.map((item) => <div key={item.view} className={styles.sheetItem}><button type="button" className={styles.sheetTop} onClick={() => go(item.view)}>{item.view === 'fabric' ? 'Fabric' : item.label}</button></div>)}
        {NAV_FLYOUT_PANELS.map((panel) => (
          <div key={panel.key} className={styles.sheetItem}>
            <button type="button" className={styles.sheetTop} aria-expanded={sheetSection === panel.key} onClick={() => setSheetSection((current) => current === panel.key ? null : panel.key)}>{panel.label}<ChevronIcon /></button>
            <div className={styles.sheetSub} data-open={String(sheetSection === panel.key)}>
              {panel.groups.flatMap((group) => group.links).map((item) => <button key={item.view} type="button" onClick={() => go(item.view)}>{item.label}</button>)}
            </div>
          </div>
        ))}
        <div className={styles.sheetUtilities}>
          {onSearchClick ? <button type="button" className={styles.searchBtn} onClick={() => { setSheetOpen(false); onSearchClick(); }}><Search size={15} />Search</button> : null}
          {onOpenHelp ? <button type="button" className={styles.searchBtn} onClick={() => { setSheetOpen(false); onOpenHelp(); }}><CircleHelp size={15} />Help</button> : null}
          <button type="button" className={styles.searchBtn} onClick={() => go('settings')}><Settings size={15} />Settings</button>
          <button type="button" className={styles.searchBtn} onClick={onLogout}><LogOut size={15} />Sign out</button>
        </div>
      </div>
    </>
  );
}

export default GlobalNav;
