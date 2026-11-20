// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

import { withAuroraPage } from '../layout/AuroraPage';
import MigrationPlannerPanel from '../MigrationPlannerPanel';

function MigPlannerPage() {
  return (
    <div className="apple-chapter">
      <MigrationPlannerPanel />
    </div>
  );
}

export default withAuroraPage('mig-planner', MigPlannerPage);
