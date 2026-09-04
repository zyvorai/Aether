// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { AIStudio } from './AIPage';

function AiAdvisorPage({ refreshKey }: { refreshKey?: number }) {
  return <AIStudio refreshKey={refreshKey} forcedTab="advisor" />;
}

export default withAuroraPage('ai-advisor', AiAdvisorPage);
