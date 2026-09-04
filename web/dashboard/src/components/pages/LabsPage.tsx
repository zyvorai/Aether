// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { withAuroraPage } from '../layout/AuroraPage';
import SectionHubPage from '../SectionHubPage';
import { FileCode2, FlaskConical, GitBranch, Layers, Network, Rocket } from 'lucide-react';

function LabsPage() {
  return (
    <SectionHubPage
      links={[
        {
          view: 'labs-live',
          title: 'Live Labs',
          description: 'Experimental live lab sessions.',
          icon: <FlaskConical className="h-5 w-5" />,
        },
        {
          view: 'labs-graduation',
          title: 'Graduation',
          description: 'Graduate lab artifacts to production.',
          icon: <Rocket className="h-5 w-5" />,
        },
        {
          view: 'labs-graph',
          title: 'Knowledge Graph',
          description: 'Infrastructure knowledge graph.',
          icon: <GitBranch className="h-5 w-5" />,
        },
        {
          view: 'labs-platform',
          title: 'Graph Platform',
          description: 'Graph platform controls.',
          icon: <Network className="h-5 w-5" />,
        },
        {
          view: 'editor',
          title: 'Visual Editor',
          description: 'Form-based workload designer.',
          icon: <Layers className="h-5 w-5" />,
        },
        {
          view: 'templates',
          title: 'Templates',
          description: 'Workload template library.',
          icon: <FileCode2 className="h-5 w-5" />,
        },
      ]}
    />
  );
}

export default withAuroraPage('labs', LabsPage);
