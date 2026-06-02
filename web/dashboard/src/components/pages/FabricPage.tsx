// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { FabricPageContent } from '../RuntimeFabricGraph';
import DigitalTwinPanel from '../DigitalTwinPanel';
import KnowledgeGraphPanel from '../KnowledgeGraphPanel';
import UnifiedFabricPanel from '../UnifiedFabricPanel';
import HubPageToc from '../HubPageToc';

const TOC = [
  { id: 'fabric-twin', label: 'Digital twin' },
  { id: 'fabric-topology', label: 'Topology' },
  { id: 'fabric-graph', label: 'Knowledge graph' },
  { id: 'fabric-unified', label: 'Unified fabric' },
];

export default function FabricPage() {
  return (
    <section className="hub-page-shell">
      <HubPageToc items={TOC} />
      <div id="fabric-twin"><DigitalTwinPanel /></div>
      <div id="fabric-topology"><FabricPageContent /></div>
      <div id="fabric-graph"><KnowledgeGraphPanel /></div>
      <div id="fabric-unified"><UnifiedFabricPanel /></div>
    </section>
  );
}
