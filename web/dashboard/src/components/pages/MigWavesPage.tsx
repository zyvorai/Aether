// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import MigrationWavePanel from '../MigrationWavePanel';

function MigWavesPage() {
  return (
    <div className="apple-chapter">
      <MigrationWavePanel />
    </div>
  );
}

export default withAuroraPage('mig-waves', MigWavesPage);
