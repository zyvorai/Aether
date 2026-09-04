// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
