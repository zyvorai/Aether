// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { withAuroraPage } from '../layout/AuroraPage';
import DigitalTwinPanel from '../DigitalTwinPanel';

function FabricTwinPage() {
  return (
    <div className="apple-chapter">
      <DigitalTwinPanel />
    </div>
  );
}

export default withAuroraPage('fabric-twin', FabricTwinPage);
