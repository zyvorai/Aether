// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

import { withAuroraPage } from '../layout/AuroraPage';
import { IntelligenceStudio } from './IntelligencePage';

function IntelEvolutionPage({ refreshKey }: { refreshKey?: number }) {
  return <IntelligenceStudio refreshKey={refreshKey} forcedTab="evolution" />;
}

export default withAuroraPage('intel-evolution', IntelEvolutionPage);
