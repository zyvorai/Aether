// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { Download, Play, Store } from 'lucide-react';
import { apiFetch, apiPost } from '../utils/api';

interface ZyraPrompt {
  id: string;
  title: string;
  category: string;
  body: string;
}

interface MarketplaceAgent {
  id: string;
  name: string;
  description: string;
  installed: boolean;
}

export default function ZyraPromptLibrary() {
  const [prompts, setPrompts] = useState<ZyraPrompt[]>([]);
  const [marketplace, setMarketplace] = useState<MarketplaceAgent[]>([]);

  useEffect(() => {
    void apiFetch<{ prompts: ZyraPrompt[] }>('/zyra/prompts').then((lib) => {
      if (lib?.prompts) setPrompts(lib.prompts);
    });
    void apiFetch<{ agents: MarketplaceAgent[] }>('/zyra/marketplace').then((m) => {
      if (m?.agents) setMarketplace(m.agents);
    });
  }, []);

  const runPrompt = (body: string) => {
    window.dispatchEvent(new CustomEvent('zyra-send', { detail: { message: body } }));
  };

  const install = async (id: string) => {
    await apiPost('/zyra/marketplace/install', { agent_id: id });
    const m = await apiFetch<{ agents: MarketplaceAgent[] }>('/zyra/marketplace');
    if (m?.agents) setMarketplace(m.agents);
  };

  return (
    <div className="grid gap-6 lg:grid-cols-2" data-testid="zyra-prompt-library">
      <div>
        <h3 className="mb-3 text-sm font-semibold text-foreground">Prompt Library</h3>
        <div className="space-y-2">
          {prompts.map((p) => (
            <div key={p.id} className="rounded-xl border border-white/10 p-3">
              <p className="text-sm font-medium text-foreground">{p.title}</p>
              <p className="mt-1 text-xs text-subtle">{p.body}</p>
              <button type="button" className="btn-secondary mt-2 text-xs" onClick={() => runPrompt(p.body)}>
                <Play className="mr-1 inline h-3 w-3" />
                Run with Zyra
              </button>
            </div>
          ))}
        </div>
      </div>
      <div>
        <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold text-foreground">
          <Store className="h-4 w-4" />
          Agent Marketplace
        </h3>
        <div className="space-y-2">
          {marketplace.map((a) => (
            <div key={a.id} className="rounded-xl border border-white/10 p-3">
              <p className="text-sm font-medium text-foreground">{a.name}</p>
              <p className="mt-1 text-xs text-subtle">{a.description}</p>
              {a.installed ? (
                <span className="mt-2 inline-block text-xs text-success">Installed</span>
              ) : (
                <button type="button" className="btn-secondary mt-2 text-xs" onClick={() => void install(a.id)}>
                  <Download className="mr-1 inline h-3 w-3" />
                  Install
                </button>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
