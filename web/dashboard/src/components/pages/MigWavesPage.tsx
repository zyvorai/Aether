// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
