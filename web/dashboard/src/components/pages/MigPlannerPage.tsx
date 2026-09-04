// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
