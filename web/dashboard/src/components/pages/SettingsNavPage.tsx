// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import NavPreferencesPanel from '../NavPreferencesPanel';

function SettingsNavPage() {
  return (
    <div className="apple-chapter">
      <NavPreferencesPanel />
    </div>
  );
}

export default withAuroraPage('settings-nav', SettingsNavPage);
