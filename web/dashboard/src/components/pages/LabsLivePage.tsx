// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
