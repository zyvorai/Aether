// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
