// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { GitOpsStudio } from './GitOpsPage';

function GitopsIntentPage({ refreshKey }: { refreshKey?: number }) {
  return <GitOpsStudio refreshKey={refreshKey} forcedSection="intent" />;
}

export default withAuroraPage('gitops-intent', GitopsIntentPage);
