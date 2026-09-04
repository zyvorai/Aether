// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { CostStudio } from './CostPage';

function CostEstimatePage({ refreshKey }: { refreshKey?: number }) {
  return <CostStudio refreshKey={refreshKey} forcedSection="estimate" />;
}

export default withAuroraPage('cost-estimate', CostEstimatePage);
