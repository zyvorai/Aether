// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import KnowledgeGraphPanel from '../KnowledgeGraphPanel';

function LabsGraphPage() {
  return (
    <div className="apple-chapter">
      <KnowledgeGraphPanel />
    </div>
  );
}

export default withAuroraPage('labs-graph', LabsGraphPage);
