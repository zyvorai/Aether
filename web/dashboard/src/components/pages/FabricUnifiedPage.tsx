// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import UnifiedFabricPanel from '../UnifiedFabricPanel';

function FabricUnifiedPage() {
  return (
    <div className="apple-chapter">
      <UnifiedFabricPanel />
    </div>
  );
}

export default withAuroraPage('fabric-unified', FabricUnifiedPage);
