// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import MacOSPlatformPanel from '../MacOSPlatformPanel';

function SettingsMacosPage() {
  return (
    <div className="apple-chapter">
      <MacOSPlatformPanel />
    </div>
  );
}

export default withAuroraPage('settings-macos', SettingsMacosPage);
