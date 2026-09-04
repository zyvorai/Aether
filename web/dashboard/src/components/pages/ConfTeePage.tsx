// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
import { withAuroraPage } from '../layout/AuroraPage';
import { ConfidentialStudio } from './ConfidentialPage';

function ConfTeePage({ refreshKey }: { refreshKey?: number }) {
  return <ConfidentialStudio refreshKey={refreshKey} forcedSection="tee" />;
}

export default withAuroraPage('conf-tee', ConfTeePage);
