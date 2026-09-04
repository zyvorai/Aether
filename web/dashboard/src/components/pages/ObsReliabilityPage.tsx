// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { withAuroraPage } from '../layout/AuroraPage';
import SreReliabilityPanel from '../SreReliabilityPanel';

function ObsReliabilityPage() {
  return (
    <div className="apple-chapter">
      <SreReliabilityPanel />
    </div>
  );
}

export default withAuroraPage('obs-reliability', ObsReliabilityPage);
