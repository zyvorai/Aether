// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AIStudio } from './AIPage';

function AiIntentPage({ refreshKey }: { refreshKey?: number }) {
  return <AIStudio refreshKey={refreshKey} forcedTab="intent" />;
}

export default withAuroraPage('ai-intent', AiIntentPage);
