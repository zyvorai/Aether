// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { LayoutGrid } from 'lucide-react';
import { useClassicNav } from '../hooks/useProView';
import { isMacOSShell } from '../utils/macosBridge';
import Badge from './Badge';
import GlassSection from './GlassSection';

export default function NavPreferencesPanel() {
  const [classicNav, setClassicNav] = useClassicNav();
  const inShell = isMacOSShell();

  return (
    <GlassSection
      accent="blue"
      testId="nav-preferences-panel"
      title="Navigation"
      subtitle="AI Infrastructure OS is the default experience. Switch to Classic CloudOS when you need legacy table-first flows."
      icon={<LayoutGrid className="h-5 w-5 text-sky-400" />}
      actions={
        inShell ? (
          <Badge text="macOS shell — AI OS locked" variant="green" />
        ) : (
          <Badge text={classicNav ? 'Classic CloudOS' : 'AI Infrastructure OS'} variant={classicNav ? 'muted' : 'green'} />
        )
      }
    >
      <label className="flex cursor-pointer items-start gap-3 rounded-xl border glass-divider px-4 py-3">
        <input
          type="checkbox"
          checked={classicNav}
          disabled={inShell}
          onChange={(e) => setClassicNav(e.target.checked)}
          className="mt-1 accent-aether"
          aria-label="Use Classic CloudOS navigation"
          data-testid="classic-nav-toggle"
        />
        <span>
          <span className="block text-sm font-medium text-slate-200">Classic CloudOS navigation</span>
          <span className="mt-1 block text-xs text-slate-500">
            {inShell
              ? 'The macOS app always uses the 12-section AI OS layout.'
              : 'Reloads the dashboard with legacy Pro/CloudOS navigation and table-first pages.'}
          </span>
        </span>
      </label>
    </GlassSection>
  );
}
