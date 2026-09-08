// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
