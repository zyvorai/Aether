// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

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
