// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
