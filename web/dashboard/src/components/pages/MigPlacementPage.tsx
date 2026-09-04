// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
