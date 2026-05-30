// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { FabricPageContent } from '../RuntimeFabricGraph';
import DigitalTwinPanel from '../DigitalTwinPanel';
import UnifiedFabricPanel from '../UnifiedFabricPanel';

export default function FabricPage() {
  return (
    <div className="space-y-8">
      <DigitalTwinPanel />
      <UnifiedFabricPanel />
      <FabricPageContent />
    </div>
  );
}
