// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { AIStudio } from './AIPage';

function AiAnalyzePage({ refreshKey }: { refreshKey?: number }) {
  return <AIStudio refreshKey={refreshKey} forcedTab="analyze" />;
}

export default withAuroraPage('ai-analyze', AiAnalyzePage);
