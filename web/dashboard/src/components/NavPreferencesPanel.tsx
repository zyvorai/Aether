// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { LayoutGrid } from 'lucide-react';
import { isMacOSShell } from '../utils/macosBridge';
import Badge from './Badge';
import GlassSection from './GlassSection';

export default function NavPreferencesPanel() {
  const inShell = isMacOSShell();

  return (
    <GlassSection
      accent="blue"
      testId="nav-preferences-panel"
      title="Navigation"
      subtitle="Aurora-style global navigation with flyouts for Intelligence, Operations, and Resources."
      icon={<LayoutGrid className="h-5 w-5 text-primary" />}
      actions={
        inShell ? (
          <Badge text="macOS shell" variant="green" />
        ) : (
          <Badge text="Global nav" variant="green" />
        )
      }
    >
      <p className="text-sm text-muted">
        {inShell
          ? 'The macOS app uses the same sticky global navigation as the web dashboard.'
          : 'Use the top bar and flyout menus to reach every dashboard view. Command palette (⌘K) provides quick jumps.'}
      </p>
    </GlassSection>
  );
}
