// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import GraphPlatformPanel from '../GraphPlatformPanel';

function LabsPlatformPage() {
  return (
    <div className="apple-chapter">
      <GraphPlatformPanel />
    </div>
  );
}

export default withAuroraPage('labs-platform', LabsPlatformPage);
