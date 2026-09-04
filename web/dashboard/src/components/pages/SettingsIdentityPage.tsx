// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
