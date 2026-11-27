// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { LayoutGrid, X } from 'lucide-react';
import { Link } from 'react-router';
import { usePreferences } from '../hooks/usePreferences';
import { useTheme, type AppTheme } from '../contexts/ThemeContext';
import { DASHBOARD_VIEWS } from '../utils/dashboardNav';
import SegmentedControl from './ui/SegmentedControl';
import GlassSection from './GlassSection';

function Row({ label, help, children }: { label: string; help: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2 border-t border-border py-3 first:border-t-0 first:pt-0 sm:flex-row sm:items-center sm:justify-between">
      <div className="min-w-0">
        <p className="text-sm font-medium text-foreground">{label}</p>
        <p className="text-xs text-muted">{help}</p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

/** Interface preferences. They are stored in this browser and change nothing on the server. */
export default function NavPreferencesPanel() {
  const { pinnedViews, density, sidebarCollapsed, welcomeDismissed, setPreferences, togglePin } = usePreferences();
  const { theme, setTheme } = useTheme();
  const pinned = pinnedViews.flatMap((view) => DASHBOARD_VIEWS.filter((meta) => meta.view === view));

  return (
    <GlassSection
      accent="blue"
      testId="nav-preferences-panel"
      title="Appearance and navigation"
      subtitle="Saved in this browser only."
      icon={<LayoutGrid className="h-5 w-5 text-primary" />}
    >
      <Row label="Theme" help="System follows your operating system.">
        <SegmentedControl<AppTheme>
          label="Theme"
          value={theme}
          onChange={setTheme}
          items={[
            { key: 'light', label: 'Light' },
            { key: 'dark', label: 'Dark' },
            { key: 'system', label: 'System' },
          ]}
        />
      </Row>
      <Row label="Density" help="Compact fits more rows on screen.">
        <SegmentedControl
          label="Density"
          value={density}
          onChange={(value) => setPreferences({ density: value })}
          items={[
            { key: 'comfortable', label: 'Comfortable' },
            { key: 'compact', label: 'Compact' },
          ]}
        />
      </Row>
      <Row label="Sidebar" help="Press [ anywhere to switch.">
        <SegmentedControl
          label="Sidebar"
          value={sidebarCollapsed ? 'rail' : 'full'}
          onChange={(value) => setPreferences({ sidebarCollapsed: value === 'rail' })}
          items={[
            { key: 'full', label: 'Full' },
            { key: 'rail', label: 'Icons only' },
          ]}
        />
      </Row>
      <Row label="Pinned pages" help="Pin from the sidebar, the breadcrumb star, or with the p key.">
        {pinned.length === 0 ? (
          <span className="text-sm text-subtle">None yet</span>
        ) : (
          <ul className="flex max-w-md flex-wrap justify-end gap-1.5">
            {pinned.map((meta) => (
              <li key={meta.view} className="inline-flex items-center gap-1 rounded-full border border-border py-1 pl-3 pr-1 text-xs">
                <Link to={meta.path} className="text-foreground hover:text-primary">{meta.label}</Link>
                <button
                  type="button"
                  onClick={() => togglePin(meta.view)}
                  aria-label={`Unpin ${meta.label}`}
                  className="grid h-5 w-5 place-items-center rounded-full text-subtle hover:bg-hover hover:text-foreground"
                >
                  <X className="h-3 w-3" aria-hidden />
                </button>
              </li>
            ))}
          </ul>
        )}
      </Row>
      <Row label="Orientation card" help="The getting-around tips shown on the Overview.">
        {welcomeDismissed ? (
          <button type="button" className="btn-quiet" onClick={() => setPreferences({ welcomeDismissed: false })}>
            Show it again
          </button>
        ) : (
          <span className="text-sm text-subtle">Visible</span>
        )}
      </Row>
    </GlassSection>
  );
}
