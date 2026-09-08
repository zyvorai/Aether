// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import SelfHealingPanel from '../SelfHealingPanel';

function ObsSelfHealingPage() {
  return (
    <div className="apple-chapter">
      <SelfHealingPanel />
    </div>
  );
}

export default withAuroraPage('obs-self-healing', ObsSelfHealingPage);
