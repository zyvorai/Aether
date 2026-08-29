import { withAuroraPage } from '../layout/AuroraPage';
// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { Link } from 'react-router';
import { FabricPageContent } from '../RuntimeFabricGraph';
import DigitalTwinPanel from '../DigitalTwinPanel';
import KnowledgeGraphPanel from '../KnowledgeGraphPanel';
import UnifiedFabricPanel from '../UnifiedFabricPanel';
import HubPageToc from '../HubPageToc';
import { WorkloadContextBanner, WorkloadScopedCrossLinks } from '../QueryContextBanner';
import { viewToPath } from '../../utils/dashboardRoutes';
import { pathWithQuery, useQueryParam } from '../../utils/urlState';

const TOC = [
  { id: 'fabric-twin', label: 'Digital twin' },
  { id: 'fabric-topology', label: 'Topology' },
  { id: 'fabric-graph', label: 'Knowledge graph' },
  { id: 'fabric-unified', label: 'Unified fabric' },
];

function FabricPage() {
  const [workload] = useQueryParam('workload');

  return (
    <section className="glass">
      {workload.trim() ? (
        <WorkloadContextBanner testId="fabric-workload-context" workload={workload} description="Fabric context">
          <WorkloadScopedCrossLinks workload={workload} prefix="fabric" showMetrics showDrift />
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('fleet'), { workload: workload.trim() })}
            className="text-brand hover:underline"
            data-testid="fabric-context-fleet-link"
          >
            Fleet →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('migrations'), { workload: workload.trim() })}
            className="text-brand hover:underline"
            data-testid="fabric-context-migrations-link"
          >
            Migrations →
          </Link>
          {' · '}
          <Link
            to={pathWithQuery(viewToPath('zyra'), { workload: workload.trim(), q: `Fabric analysis for ${workload.trim()}` })}
            className="text-brand hover:underline"
            data-testid="fabric-context-zyra-link"
          >
            Zyra →
          </Link>
        </WorkloadContextBanner>
      ) : null}
      <HubPageToc items={TOC} />
      <div id="fabric-twin"><DigitalTwinPanel /></div>
      <div id="fabric-topology"><FabricPageContent /></div>
      <div id="fabric-graph"><KnowledgeGraphPanel /></div>
      <div id="fabric-unified"><UnifiedFabricPanel /></div>
    </section>
  );
}

export default withAuroraPage('fabric', FabricPage);
