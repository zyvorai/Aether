import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { Link } from 'react-router';
import SectionHubPage from '../SectionHubPage';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { ArrowRightLeft, HardDrive, Layers, MapPin } from 'lucide-react';

function MigrationsPage() {
  const [workload] = useQueryParam('workload');

  return (
    <div className="space-y-12">
      <div className="flex flex-wrap gap-x-6 gap-y-2 text-sm text-muted" data-testid="migrations-hub-context">
        <span>Migrations</span>
        <Link to={viewToPath('health')} className="text-primary hover:underline" data-testid="migrations-context-orchestrator-link">
          Orchestrator →
        </Link>
        <Link to={viewToPath('security')} className="text-primary hover:underline" data-testid="migrations-context-security-link">
          Security →
        </Link>
        <Link to={viewToPath('gitops')} className="text-primary hover:underline" data-testid="migrations-context-gitops-link">
          GitOps →
        </Link>
        <Link to={viewToPath('hosted')} className="text-primary hover:underline" data-testid="migrations-context-hosted-link">
          Hosted SaaS →
        </Link>
      </div>
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
      <SectionHubPage
      links={[
          {
            view: 'mig-planner',
            title: 'Migration Planner',
            description: 'AI migration planner with risk analysis.',
            icon: <ArrowRightLeft className="h-5 w-5" />,
          },
          {
            view: 'mig-placement',
            title: 'Autonomous Placement',
            description: 'Autonomous placement recommendations.',
            icon: <MapPin className="h-5 w-5" />,
          },
          {
            view: 'mig-replication',
            title: 'Volume Replication',
            description: 'Volume replication for migrations.',
            icon: <HardDrive className="h-5 w-5" />,
          },
          {
            view: 'mig-waves',
            title: 'Migration Waves',
            description: 'Wave-based migration orchestration.',
            icon: <Layers className="h-5 w-5" />,
          },
        ]}
      />
    </div>
  );
}

export default withAuroraPage('migrations', MigrationsPage);
