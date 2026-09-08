// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
