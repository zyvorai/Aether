// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import AutonomousSrePanel from '../AutonomousSrePanel';

function ObsAutonomousSrePage() {
  return (
    <div className="apple-chapter">
      <AutonomousSrePanel />
    </div>
  );
}

export default withAuroraPage('obs-autonomous-sre', ObsAutonomousSrePage);
