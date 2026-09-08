// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import CapacityScalePanel from '../CapacityScalePanel';

function ObsCapacityScalePage() {
  return (
    <div className="apple-chapter">
      <CapacityScalePanel />
    </div>
  );
}

export default withAuroraPage('obs-capacity-scale', ObsCapacityScalePage);
