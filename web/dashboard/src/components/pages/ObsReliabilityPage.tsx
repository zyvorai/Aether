// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
