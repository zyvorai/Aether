// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { ArrowRight } from 'lucide-react';
import type { AppView } from '../types/api';
import { viewToPath } from '../utils/dashboardRoutes';
import { Link } from 'react-router';
import type { Tone } from './layout/PageHero';

export interface HubLink {
  view: AppView;
  title: string;
  description: string;
  icon: ReactNode;
  /** Defaults to a cycling tone by position when omitted. */
  tone?: Tone;
}

interface SectionHubPageProps {
  /** Optional; omit when AuroraPage already provides the page title. */
  title?: string;
  subtitle?: string;
  links: HubLink[];
}

/** Tone rotation for hub bands that don't specify one — kept in Apple.com/services
 * order (one distinct color per destination), capped at DESIGN.md's ~6 rows. */
const TONE_CYCLE: Tone[] = ['sky', 'violet', 'emerald', 'amber', 'pink', 'teal', 'rust'];

/** Services-style hub — one full-width, tone-tinted band per destination. */
export default function SectionHubPage({ title, subtitle, links }: SectionHubPageProps) {
  return (
    <section className="apple-chapter">
      {title || subtitle ? (
        <div className="mb-10 max-w-2xl space-y-2">
          {title ? <h2 className="text-2xl font-semibold tracking-[var(--tracking-display)] text-foreground sm:text-3xl">{title}</h2> : null}
          {subtitle ? <p className="text-base text-muted">{subtitle}</p> : null}
        </div>
      ) : null}
      <div className="hub-band-list">
        {links.map((link, index) => (
          <Link
            key={link.view}
            to={viewToPath(link.view)}
            data-tone={link.tone ?? TONE_CYCLE[index % TONE_CYCLE.length]}
            className="hub-band"
          >
            <div className="hub-band-icon">{link.icon}</div>
            <div className="min-w-0 flex-1 space-y-1">
              <h3 className="text-lg font-semibold tracking-[var(--tracking-display)] text-foreground">
                {link.title}
              </h3>
              <p className="max-w-xl text-sm leading-relaxed text-muted">{link.description}</p>
            </div>
            <ArrowRight className="hub-band-arrow h-4 w-4 shrink-0" />
          </Link>
        ))}
      </div>
    </section>
  );
}
