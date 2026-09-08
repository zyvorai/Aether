// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import KnowledgeGraphPanel from '../KnowledgeGraphPanel';

function FabricGraphPage() {
  return (
    <div className="apple-chapter">
      <KnowledgeGraphPanel />
    </div>
  );
}

export default withAuroraPage('fabric-graph', FabricGraphPage);
