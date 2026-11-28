// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { ExternalLink } from 'lucide-react';
import { ZYVOR_URL, ZYVOR_BRAND, ZYVOR_COPY, ZYVOR_LINE } from './ZyvorBrand';
import { ZYVOR_HELP, AETHER_HELP } from '../config/zyvorHelp';
import { AetherIcon } from './brand';

export const AETHER_PRODUCT = AETHER_HELP.name;
export const AETHER_VERSION = AETHER_HELP.version;
export const AETHER_TAGLINE = `${AETHER_HELP.tagline} — one spec, three runtimes, one tool.`;

const ORANGE = '#f97316';

export type HelpDocLink = {
  label: string;
  href: string;
};

export const AETHER_HELP_LINKS: HelpDocLink[] = [
  {
    label: 'Documentation index',
    href: 'https://github.com/zyvorai/Aether/blob/main/docs/README.md',
  },
  {
    label: 'Web dashboard guide',
    href: 'https://github.com/zyvorai/Aether/tree/main/web/dashboard',
  },
  {
    label: 'CLI & API reference',
    href: 'https://github.com/zyvorai/Aether/blob/main/README.md',
  },
  {
    label: 'Customer bundle help (HELP.txt)',
    href: 'https://github.com/zyvorai/Aether/blob/main/scripts/lib/write-customer-help.sh',
  },
  {
    label: 'Zyvor documentation',
    href: ZYVOR_HELP.docs,
  },
  {
    label: 'Aether on zyvor.dev',
    href: AETHER_HELP.productUrl,
  },
  {
    label: 'Zyvor — HyperSDK suite',
    href: ZYVOR_URL,
  },
];

export default function ZyvorAbout({ className = '' }: { className?: string }) {

  return (
    <div className={`space-y-5 text-sm ${'text-muted'} ${className}`.trim()}>
      <div className="flex items-start gap-4">
        <AetherIcon size={56} className="shrink-0" />
        <div className="min-w-0 pt-0.5">
          <h3 className={`text-lg font-semibold ${'text-foreground'}`}>{AETHER_PRODUCT}</h3>
          <p className="text-xs text-subtle mt-0.5">Version {AETHER_VERSION}</p>
          <p className={`text-sm mt-2 leading-relaxed ${'text-muted'}`}>{AETHER_TAGLINE}</p>
        </div>
      </div>

      <div className={`rounded-xl border p-4 space-y-3 ${'glass'}`}>
        <p className="leading-relaxed">
          Part of the{' '}
          <a
            href={ZYVOR_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="font-semibold hover:underline"
            style={{ color: ORANGE }}
          >
            {ZYVOR_BRAND}
          </a>{' '}
          product family — deploy workloads to Podman, Kubernetes, and KubeVirt from a single YAML
          specification with AI-assisted runtime selection and built-in observability.
        </p>
        <p className="text-xs text-subtle leading-relaxed">
          <span style={{ color: ORANGE }} className="font-medium">
            {ZYVOR_LINE}
          </span>
          <br />
          Source-available under the Zyvor Production Licence v2.0 — see the repository LICENSE.
          Confidential computing ships as Ragnarok, a separate Zyvor product.
        </p>
      </div>

      <div>
        <h4 className="text-xs font-semibold uppercase tracking-wider text-subtle mb-2">Help & documentation</h4>
        <ul className="space-y-1.5">
          {AETHER_HELP_LINKS.map((link) => (
            <li key={link.href}>
              <a
                href={link.href}
                target="_blank"
                rel="noopener noreferrer"
                className={`inline-flex items-center gap-1.5 hover:text-primary transition-colors ${'text-muted'}`}
              >
                <span>{link.label}</span>
                <ExternalLink className="w-3.5 h-3.5 shrink-0 opacity-60" aria-hidden />
              </a>
            </li>
          ))}
        </ul>
      </div>

      <p className={`text-center text-xs text-subtle pt-2 border-t ${'glass-divider/50'}`}>
        {ZYVOR_COPY} {ZYVOR_BRAND}. All rights reserved.
      </p>
    </div>
  );
}
