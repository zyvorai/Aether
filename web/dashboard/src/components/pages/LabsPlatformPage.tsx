// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
