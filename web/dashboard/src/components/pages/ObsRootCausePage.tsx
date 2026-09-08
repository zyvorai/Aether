// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import FleetRootCausePanel from '../FleetRootCausePanel';

function ObsRootCausePage() {
  return (
    <div className="apple-chapter">
      <FleetRootCausePanel />
    </div>
  );
}

export default withAuroraPage('obs-root-cause', ObsRootCausePage);
