// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { withAuroraPage } from '../layout/AuroraPage';
import { AIStudio } from './AIPage';

function AiOptimizePage({ refreshKey }: { refreshKey?: number }) {
  return <AIStudio refreshKey={refreshKey} forcedTab="optimize" />;
}

export default withAuroraPage('ai-optimize', AiOptimizePage);
