// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { FabricPageContent } from '../RuntimeFabricGraph';

function FabricTopologyPage() {
  return (
    <div className="apple-chapter">
      <FabricPageContent />
    </div>
  );
}

export default withAuroraPage('fabric-topology', FabricTopologyPage);
