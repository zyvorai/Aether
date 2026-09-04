// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { withAuroraPage } from '../layout/AuroraPage';
import VolumeReplicationPanel from '../VolumeReplicationPanel';

function MigReplicationPage() {
  return (
    <div className="apple-chapter">
      <VolumeReplicationPanel />
    </div>
  );
}

export default withAuroraPage('mig-replication', MigReplicationPage);
