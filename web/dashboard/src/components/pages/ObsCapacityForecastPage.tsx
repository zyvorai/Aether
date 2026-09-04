// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
