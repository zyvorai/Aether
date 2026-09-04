// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { AIStudio } from './AIPage';

function AiRecommendPage({ refreshKey }: { refreshKey?: number }) {
  return <AIStudio refreshKey={refreshKey} forcedTab="recommend" />;
}

export default withAuroraPage('ai-recommend', AiRecommendPage);
