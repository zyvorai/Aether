// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import AutonomousModePanel from '../AutonomousModePanel';

function SettingsAutonomousPage() {
  return (
    <div className="apple-chapter">
      <AutonomousModePanel />
    </div>
  );
}

export default withAuroraPage('settings-autonomous', SettingsAutonomousPage);
