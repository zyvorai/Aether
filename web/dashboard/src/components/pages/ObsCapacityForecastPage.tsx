// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import CapacityForecastPanel from '../CapacityForecastPanel';

function ObsCapacityForecastPage() {
  return (
    <div className="apple-chapter">
      <CapacityForecastPanel />
    </div>
  );
}

export default withAuroraPage('obs-capacity-forecast', ObsCapacityForecastPage);
