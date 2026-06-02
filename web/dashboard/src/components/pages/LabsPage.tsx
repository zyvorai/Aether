// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import KnowledgeGraphPanel from '../KnowledgeGraphPanel';
import GraphPlatformPanel from '../GraphPlatformPanel';
import LabsGraduationPanel from '../LabsGraduationPanel';
import LiveLabsPanel from '../LiveLabsPanel';
import SectionHubPage from '../SectionHubPage';
import HubPageToc from '../HubPageToc';
import { FileCode2, FlaskConical, GitBranch, Layers, Terminal } from 'lucide-react';

const TOC = [
  { id: 'labs-live', label: 'Live labs' },
  { id: 'labs-graduation', label: 'Graduation' },
  { id: 'labs-graph', label: 'Knowledge graph' },
  { id: 'labs-platform', label: 'Graph platform' },
  { id: 'labs-tools', label: 'Tools' },
];

export default function LabsPage() {
  return (
    <section className="hub-page-shell">
      <HubPageToc items={TOC} />
      <div id="labs-live"><LiveLabsPanel /></div>
      <div id="labs-graduation"><LabsGraduationPanel /></div>
      <div id="labs-graph"><KnowledgeGraphPanel /></div>
      <div id="labs-platform"><GraphPlatformPanel /></div>
      <div id="labs-tools">
        <SectionHubPage
          title="Labs"
          subtitle="Experimental AI features — generated Helm charts, Terraform, and runbooks."
          links={[
            {
              view: 'helm',
              title: 'AI Helm Charts',
              description: 'Deploy curated charts with a guided wizard.',
              icon: <FileCode2 className="h-5 w-5" />,
            },
            {
              view: 'compose',
              title: 'Compose Import',
              description: 'Import Docker Compose into Aether workloads.',
              icon: <Layers className="h-5 w-5" />,
            },
            {
              view: 'deps',
              title: 'Dependency Graph',
              description: 'Workload dependency visualization.',
              icon: <GitBranch className="h-5 w-5" />,
            },
            {
              view: 'openapi',
              title: 'API Explorer',
              description: 'Browse OpenAPI routes and raw schema.',
              icon: <Terminal className="h-5 w-5" />,
            },
            {
              view: 'templates',
              title: 'Templates',
              description: 'Workload template library for quick experiments.',
              icon: <FlaskConical className="h-5 w-5" />,
            },
          ]}
        />
      </div>
    </section>
  );
}
