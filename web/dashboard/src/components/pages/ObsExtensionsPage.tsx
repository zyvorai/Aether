// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { withAuroraPage } from '../layout/AuroraPage';
import ExtensionsGraduationPanel from '../ExtensionsGraduationPanel';

function ObsExtensionsPage() {
  return (
    <div className="apple-chapter">
      <ExtensionsGraduationPanel />
    </div>
  );
}

export default withAuroraPage('obs-extensions', ObsExtensionsPage);
