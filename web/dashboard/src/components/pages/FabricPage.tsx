import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { Link } from 'react-router';
import SectionHubPage from '../SectionHubPage';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';
import { GitBranch, Globe, Network, Sparkles } from 'lucide-react';

function FabricPage() {
  const [workload] = useQueryParam('workload');

  return (
    <div className="space-y-12">
      {workload.trim() ? (
        <WorkloadContextBanner testId="fabric-workload-context" workload={workload} description="Fabric context">
          <WorkloadScopedCrossLinks workload={workload} prefix="fabric" showMetrics showDrift />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workload.trim() })}
            className="text-primary hover:underline"
            data-testid="fabric-context-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('migrations'), { workload: workload.trim() })}
            className="text-primary hover:underline"
            data-testid="fabric-context-migrations-link"
          >
            Migrations →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zyra'), { workload: workload.trim(), q: `Fabric analysis for ${workload.trim()}` })}
            className="text-primary hover:underline"
            data-testid="fabric-context-zyra-link"
          >
            Zyra →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <SectionHubPage
      links={[
          {
            view: 'fabric-twin',
            title: 'Digital Twin',
            description: 'Digital twin of the runtime fabric.',
            icon: <Sparkles className="h-5 w-5" />,
          },
          {
            view: 'fabric-topology',
            title: 'Topology',
            description: 'Application → Runtime → Cluster → Node topology.',
            icon: <Network className="h-5 w-5" />,
          },
          {
            view: 'fabric-graph',
            title: 'Knowledge Graph',
            description: 'Fabric knowledge graph.',
            icon: <GitBranch className="h-5 w-5" />,
          },
          {
            view: 'fabric-unified',
            title: 'Unified Fabric',
            description: 'Unified fabric view.',
            icon: <Globe className="h-5 w-5" />,
          },
        ]}
      />
    </div>
  );
}

export default withAuroraPage('fabric', FabricPage);
