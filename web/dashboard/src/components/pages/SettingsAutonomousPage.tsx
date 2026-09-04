// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
