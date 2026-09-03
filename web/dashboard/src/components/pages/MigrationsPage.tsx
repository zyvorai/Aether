import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { Link } from 'react-router';
import MigrationPlannerPanel from '../MigrationPlannerPanel';
import AutonomousPlacementPanel from '../AutonomousPlacementPanel';
import VolumeReplicationPanel from '../VolumeReplicationPanel';
import MigrationWavePanel from '../MigrationWavePanel';
import SectionHubPage from '../SectionHubPage';
import HubPageToc from '../HubPageToc';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { Brain, GitCompare, Sparkles } from 'lucide-react';

const TOC = [
  { id: 'mig-planner', label: 'Planner' },
  { id: 'mig-placement', label: 'Placement' },
  { id: 'mig-replication', label: 'Replication' },
  { id: 'mig-waves', label: 'Waves' },
  { id: 'mig-tools', label: 'Tools' },
];

function MigrationsPage() {
  const [workload] = useQueryParam('workload');

  const hubBanner = (
    <div className="mb-6 glass-context-banner" data-testid="migrations-hub-context">
      Migrations
      {' · '}
      <Link to={viewToPath('health')} className="text-primary hover:underline" data-testid="migrations-context-orchestrator-link">
        Orchestrator →
      </Link>
      {' · '}
      <Link to={viewToPath('security')} className="text-primary hover:underline" data-testid="migrations-context-security-link">
        Security →
      </Link>
      {' · '}
      <Link to={viewToPath('gitops')} className="text-primary hover:underline" data-testid="migrations-context-gitops-link">
        GitOps →
      </Link>
      {' · '}
      <Link to={viewToPath('hosted')} className="text-primary hover:underline" data-testid="migrations-context-hosted-link">
        Hosted SaaS →
      </Link>
    </div>
  );

  return (
    <section className="glass">
      {hubBanner}
      {workload.trim() ? (
        <WorkloadContextBanner testId="migrations-workload-context" workload={workload} description="Migration context">
          <WorkloadScopedCrossLinks workload={workload} prefix="mig" showGitops showMetrics />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fabric'), { workload: workload.trim() })}
            className="text-primary hover:underline"
            data-testid="mig-context-fabric-link"
          >
            Fabric →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workload.trim() })}
            className="text-primary hover:underline"
            data-testid="mig-context-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('drift'), { workload: workload.trim() })}
            className="text-primary hover:underline"
            data-testid="mig-context-drift-link"
          >
            Drift →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <HubPageToc items={TOC} />
      <div id="mig-planner"><MigrationPlannerPanel /></div>
      <div id="mig-placement"><AutonomousPlacementPanel /></div>
      <div id="mig-replication"><VolumeReplicationPanel /></div>
      <div id="mig-waves"><MigrationWavePanel /></div>
      <div id="mig-tools">
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
      </div>
    </section>
  );
}

export default withAuroraPage('migrations', MigrationsPage);
