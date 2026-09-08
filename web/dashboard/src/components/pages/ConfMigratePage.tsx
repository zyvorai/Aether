// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { ConfidentialStudio } from './ConfidentialPage';

function ConfMigratePage({ refreshKey }: { refreshKey?: number }) {
  return <ConfidentialStudio refreshKey={refreshKey} forcedSection="migrate" />;
}

export default withAuroraPage('conf-migrate', ConfMigratePage);
