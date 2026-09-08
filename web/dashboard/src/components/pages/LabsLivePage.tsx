// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import LiveLabsPanel from '../LiveLabsPanel';

function LabsLivePage() {
  return (
    <div className="apple-chapter">
      <LiveLabsPanel />
    </div>
  );
}

export default withAuroraPage('labs-live', LabsLivePage);
