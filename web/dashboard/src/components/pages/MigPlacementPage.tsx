// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import AutonomousPlacementPanel from '../AutonomousPlacementPanel';

function MigPlacementPage() {
  return (
    <div className="apple-chapter">
      <AutonomousPlacementPanel />
    </div>
  );
}

export default withAuroraPage('mig-placement', MigPlacementPage);
