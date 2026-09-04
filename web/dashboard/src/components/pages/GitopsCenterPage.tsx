// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { GitOpsStudio } from './GitOpsPage';

function GitopsCenterPage({ refreshKey }: { refreshKey?: number }) {
  return <GitOpsStudio refreshKey={refreshKey} forcedSection="center" />;
}

export default withAuroraPage('gitops-center', GitopsCenterPage);
