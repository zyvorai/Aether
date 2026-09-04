// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
