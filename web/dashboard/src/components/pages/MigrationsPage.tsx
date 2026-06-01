// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import MigrationPlannerPanel from '../MigrationPlannerPanel';
import AutonomousPlacementPanel from '../AutonomousPlacementPanel';
import VolumeReplicationPanel from '../VolumeReplicationPanel';
import MigrationWavePanel from '../MigrationWavePanel';
import SectionHubPage from '../SectionHubPage';
import { Brain, GitCompare, Sparkles } from 'lucide-react';

export default function MigrationsPage() {
  return (
    <section className="overview-section-shell mb-6 space-y-8 p-6 sm:p-8">
      <AutonomousPlacementPanel />
      <MigrationPlannerPanel />
      <VolumeReplicationPanel />
      <MigrationWavePanel />
      <SectionHubPage
        title="Migration tools"
        subtitle="Supporting views for planning, evolution, and pre-flight checks."
        links={[
          {
            view: 'ai',
            title: 'Runtime Advisor',
            description: 'Compare runtimes with confidence scores and explainability.',
            icon: <Brain className="h-5 w-5" />,
          },
          {
            view: 'intelligence',
            title: 'Evolution status',
            description: 'Fleet-wide runtime trajectory and auto-eligible migrations.',
            icon: <Sparkles className="h-5 w-5" />,
          },
          {
            view: 'drift',
            title: 'Pre-migration drift',
            description: 'Reconcile spec drift before executing a migration.',
            icon: <GitCompare className="h-5 w-5" />,
          },
        ]}
      />
    </section>
  );
}
