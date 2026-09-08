// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import IdentitySsoPanel from '../IdentitySsoPanel';

function SettingsIdentityPage() {
  return (
    <div className="apple-chapter">
      <IdentitySsoPanel />
    </div>
  );
}

export default withAuroraPage('settings-identity', SettingsIdentityPage);
