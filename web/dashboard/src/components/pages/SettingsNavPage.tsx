// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
